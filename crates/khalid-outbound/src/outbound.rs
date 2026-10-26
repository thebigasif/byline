//! The outbound HTTP client.

use std::error::Error;
use std::fmt;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use khalid_core::AbsoluteUrl;
use reqwest::RequestBuilder;
use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::redirect::{Action, Attempt, Policy};

use crate::{Reach, ResolveError, Resolver};

#[expect(
    clippy::disallowed_types,
    reason = "the one place the raw client is named; everything else goes through Outbound (BL-RUN-8)"
)]
type RawClient = reqwest::Client;

/// The limits every outbound request runs under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    timeout: Duration,
    connect_timeout: Duration,
    max_body_bytes: usize,
    max_redirects: usize,
}

impl Default for Limits {
    /// Ten seconds in all, five of them to connect, 10 MiB of body and five
    /// redirects.
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(10),
            connect_timeout: Duration::from_secs(5),
            max_body_bytes: 10 * 1024 * 1024,
            max_redirects: 5,
        }
    }
}

impl Limits {
    /// The same limits with the whole request capped at `timeout`.
    #[must_use]
    pub fn with_timeout(self, timeout: Duration) -> Self {
        Self { timeout, ..self }
    }

    /// The largest body read before the request is refused.
    pub fn max_body_bytes(&self) -> usize {
        self.max_body_bytes
    }
}

/// The one HTTP client (RULES.md BL-RUN-8): timeouts, a body size cap, a
/// [`Resolver`] that checks every address a host name resolves to, and
/// redirects followed within a cap, each through the same guard: a literal
/// address is checked before the hop, and a host name when it resolves.
///
/// Every crate that talks to another server takes an `&Outbound`. It is cheap
/// to clone.
#[derive(Debug, Clone)]
pub struct Outbound {
    client: RawClient,
    limits: Limits,
    resolver: Resolver,
}

impl Outbound {
    /// A client for public addresses, resolving names through the system.
    ///
    /// # Errors
    ///
    /// Returns [`OutboundError::Client`] when the TLS setup fails.
    pub fn new(limits: Limits) -> Result<Self, OutboundError> {
        Self::with_resolver(limits, Resolver::new())
    }

    /// A client that resolves names through `resolver`, such as a test table.
    ///
    /// # Errors
    ///
    /// Returns [`OutboundError::Client`] when the TLS setup fails.
    pub fn with_resolver(limits: Limits, resolver: Resolver) -> Result<Self, OutboundError> {
        // Installing fails when a provider is already in place, which is fine.
        drop(rustls::crypto::ring::default_provider().install_default());
        let max = limits.max_redirects;
        let reach = resolver.reach();
        let redirects = Policy::custom(move |attempt| follow(attempt, max, reach));
        let client = RawClient::builder()
            .timeout(limits.timeout)
            .connect_timeout(limits.connect_timeout)
            .redirect(redirects)
            .dns_resolver(Arc::new(Guarded(resolver.clone())))
            .no_proxy()
            .build()
            .map_err(|source| OutboundError::Client { source })?;
        Ok(Self {
            client,
            limits,
            resolver,
        })
    }

    /// Sends a GET request for `url` and reads its body within the limits.
    ///
    /// # Errors
    ///
    /// Returns [`OutboundError::Blocked`] when the host, an address it
    /// resolves to or a redirect points inward; [`OutboundError::TimedOut`],
    /// [`OutboundError::TooLarge`] or [`OutboundError::TooManyRedirects`] when
    /// a limit is passed; and
    /// [`OutboundError::Http`] for anything else.
    pub async fn get(&self, url: &AbsoluteUrl) -> Result<Fetched, OutboundError> {
        self.check_literal(url)?;
        self.send(self.client.get(url.as_str())).await
    }

    /// Sends a POST request for `url` with `headers` and `body`, and reads the
    /// answer within the limits. Redirects are followed through the same
    /// guard as for [`Outbound::get`]: a 307 or 308 repeats the POST with its
    /// body, and a 301, 302 or 303 continues as a GET, as browsers do. A
    /// caller that must see the first answer, such as a webhook delivery,
    /// treats any status other than 2xx as a failure.
    ///
    /// # Errors
    ///
    /// Returns [`OutboundError::Header`] when a header name or value is not
    /// valid HTTP, before anything is sent, and otherwise the errors
    /// [`Outbound::get`] returns.
    pub async fn post(
        &self,
        url: &AbsoluteUrl,
        headers: &[(&str, &str)],
        body: Vec<u8>,
    ) -> Result<Fetched, OutboundError> {
        let mut map = HeaderMap::with_capacity(headers.len());
        for &(name, value) in headers {
            let invalid = || OutboundError::Header {
                name: name.to_owned(),
            };
            let key = HeaderName::from_bytes(name.as_bytes()).map_err(|_| invalid())?;
            let value = HeaderValue::from_str(value).map_err(|_| invalid())?;
            map.append(key, value);
        }
        self.check_literal(url)?;
        self.send(self.client.post(url.as_str()).headers(map).body(body))
            .await
    }

    /// Refuses a URL whose host is a literal address the guard does not
    /// allow. A host name is checked when it resolves.
    fn check_literal(&self, url: &AbsoluteUrl) -> Result<(), OutboundError> {
        let host = url.host().trim_matches(['[', ']']);
        match host.parse::<IpAddr>() {
            Ok(address) if !self.resolver.reach().allows(address) => Err(OutboundError::Blocked {
                host: host.to_owned(),
                address,
            }),
            _ => Ok(()),
        }
    }

    /// Sends a prepared request and reads its body within the size cap.
    async fn send(&self, request: RequestBuilder) -> Result<Fetched, OutboundError> {
        let mut response = request.send().await.map_err(classify)?;
        let status = response.status().as_u16();
        let limit = self.limits.max_body_bytes;
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(classify)? {
            if body.len().saturating_add(chunk.len()) > limit {
                return Err(OutboundError::TooLarge { limit });
            }
            body.extend_from_slice(&chunk);
        }
        Ok(Fetched { status, body })
    }
}

/// A response read in full.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fetched {
    status: u16,
    body: Vec<u8>,
}

impl Fetched {
    /// The HTTP status code.
    pub fn status(&self) -> u16 {
        self.status
    }

    /// The body as bytes.
    pub fn body(&self) -> &[u8] {
        &self.body
    }
}

/// Follows a redirect only within the cap and only to a literal address the
/// guard allows. A host name is checked when it resolves.
fn follow(attempt: Attempt<'_>, max: usize, reach: Reach) -> Action {
    if attempt.previous().len() > max {
        return attempt.error(TooManyRedirects(max));
    }
    let host = attempt
        .url()
        .host_str()
        .unwrap_or_default()
        .trim_matches(['[', ']'])
        .to_owned();
    match host.parse::<IpAddr>() {
        Ok(address) if !reach.allows(address) => {
            attempt.error(ResolveError::Blocked { host, address })
        }
        _ => attempt.follow(),
    }
}

/// The redirect cap, carried inside a `reqwest` error.
#[derive(Debug)]
struct TooManyRedirects(usize);

impl fmt::Display for TooManyRedirects {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "more than {} redirects", self.0)
    }
}

impl Error for TooManyRedirects {}

/// The resolver as `reqwest` calls it. Its refusal travels inside the
/// `reqwest` error and [`classify`] finds it again.
struct Guarded(Resolver);

impl Resolve for Guarded {
    fn resolve(&self, name: Name) -> Resolving {
        let resolver = self.0.clone();
        Box::pin(async move {
            match resolver.lookup(name.as_str()).await {
                Ok(addresses) => {
                    let addrs: Addrs =
                        Box::new(addresses.into_iter().map(|ip| SocketAddr::new(ip, 0)));
                    Ok(addrs)
                }
                Err(error) => {
                    let error: Box<dyn Error + Send + Sync> = Box::new(error);
                    Err(error)
                }
            }
        })
    }
}

/// Finds the guard's refusals inside a `reqwest` error.
fn classify(error: reqwest::Error) -> OutboundError {
    if error.is_timeout() {
        return OutboundError::TimedOut;
    }
    let mut cause = error.source();
    while let Some(inner) = cause {
        if let Some(ResolveError::Blocked { host, address }) = inner.downcast_ref::<ResolveError>()
        {
            return OutboundError::Blocked {
                host: host.clone(),
                address: *address,
            };
        }
        if let Some(cap) = inner.downcast_ref::<TooManyRedirects>() {
            return OutboundError::TooManyRedirects { limit: cap.0 };
        }
        cause = inner.source();
    }
    OutboundError::Http { source: error }
}

/// Why an outbound request did not complete.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum OutboundError {
    /// The host, an address it resolves to, or a redirect points inward.
    #[error("{host} resolves to {address}, which is not a public address")]
    Blocked {
        /// The host name or literal address asked for.
        host: String,
        /// The address the guard refused.
        address: IpAddr,
    },
    /// The request took longer than its limit.
    #[error("the request took longer than its time limit")]
    TimedOut,
    /// The body was larger than its limit.
    #[error("the response is larger than {limit} bytes")]
    TooLarge {
        /// The limit, in bytes.
        limit: usize,
    },
    /// The request was redirected more times than allowed.
    #[error("the request was redirected more than {limit} times")]
    TooManyRedirects {
        /// The redirect cap.
        limit: usize,
    },
    /// A header name or value is not valid HTTP. Nothing was sent.
    #[error("the header {name} is not valid HTTP")]
    Header {
        /// The header name as given.
        name: String,
    },
    /// Any other failure while sending or reading, such as a name that
    /// cannot be found.
    #[error("the request failed")]
    Http {
        /// What the HTTP client reported.
        #[source]
        source: reqwest::Error,
    },
    /// The client could not be set up.
    #[error("the HTTP client could not be set up")]
    Client {
        /// What the HTTP client reported.
        #[source]
        source: reqwest::Error,
    },
}
