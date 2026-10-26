//! The outbound HTTP client.

use std::error::Error;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use khalid_core::AbsoluteUrl;
use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use reqwest::redirect::Policy;

use crate::{ResolveError, Resolver};

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
}

impl Default for Limits {
    /// Ten seconds in all, five of them to connect, and 10 MiB of body.
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(10),
            connect_timeout: Duration::from_secs(5),
            max_body_bytes: 10 * 1024 * 1024,
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

/// The one HTTP client (RULES.md BL-RUN-8): timeouts, a body size cap, and a
/// [`Resolver`] that checks every address a host name resolves to. Redirects
/// are not followed yet, so a redirect comes back as the response and cannot
/// slip past the guard.
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
        let client = RawClient::builder()
            .timeout(limits.timeout)
            .connect_timeout(limits.connect_timeout)
            .redirect(Policy::none())
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
    /// Returns [`OutboundError::Blocked`] when the host or an address it
    /// resolves to points inward, [`OutboundError::TimedOut`] or
    /// [`OutboundError::TooLarge`] when a limit is passed, and
    /// [`OutboundError::Http`] for anything else.
    pub async fn get(&self, url: &AbsoluteUrl) -> Result<Fetched, OutboundError> {
        let host = url.host().trim_matches(['[', ']']);
        if let Ok(address) = host.parse::<IpAddr>()
            && !self.resolver.reach().allows(address)
        {
            return Err(OutboundError::Blocked {
                host: host.to_owned(),
                address,
            });
        }
        let mut response = self
            .client
            .get(url.as_str())
            .send()
            .await
            .map_err(classify)?;
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

/// Finds the guard's refusal inside a `reqwest` error.
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
        cause = inner.source();
    }
    OutboundError::Http { source: error }
}

/// Why an outbound request did not complete.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum OutboundError {
    /// The host or an address it resolves to points inward.
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
