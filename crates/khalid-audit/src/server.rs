//! A running server as a crawl source, asked over HTTP.

use khalid_core::AbsoluteUrl;
use khalid_outbound::{Outbound, OutboundError};

use crate::{Answer, Headers, Source, Unreached};

/// A server that answers for the blog, such as `khalid serve` on this
/// machine or the live site, asked over HTTP.
///
/// The crawl keeps the site's own addresses, so canonicals and
/// translations still match; each request goes to `server` instead of the
/// site's host, as `curl --resolve` does, and a `Location` that names the
/// server is turned back into the site's address. The client should follow
/// no redirect, so the crawl sees each hop (ADR "the audit may reach the
/// site under audit on loopback").
#[derive(Debug, Clone)]
pub struct Server {
    outbound: Outbound,
    site: String,
    asked: String,
}

impl Server {
    /// Asks `server`, such as `http://127.0.0.1:8080/`, for the addresses of
    /// the site at `site`, such as `https://www.example.com/`, through
    /// `outbound`.
    pub fn new(outbound: Outbound, site: &AbsoluteUrl, server: &AbsoluteUrl) -> Self {
        let origin = |url: &AbsoluteUrl| {
            url::Url::parse(url.as_str())
                .map(|url| url.origin().ascii_serialization())
                .unwrap_or_default()
        };
        Self {
            outbound,
            site: origin(site),
            asked: origin(server),
        }
    }
}

impl Source for Server {
    const HEADERS: Headers = Headers::Sent;

    async fn fetch(&self, url: &AbsoluteUrl) -> Result<Answer, Unreached> {
        // Only the site's own origin, then a path: a host that merely
        // starts the same, such as `www.example.com.evil.test`, is refused
        // before the server is asked.
        let Some(path) = path_on(url.as_str(), &self.site) else {
            return Err(Unreached::Refused);
        };
        let request = http::Request::get(format!("{}{path}", self.asked))
            .body(Vec::new())
            .map_err(|_| Unreached::Failed)?;
        let (parts, body) = self
            .outbound
            .exchange(request)
            .await
            .map_err(|error| match error {
                OutboundError::TimedOut => Unreached::TimedOut,
                OutboundError::TooLarge { .. } => Unreached::TooLarge,
                OutboundError::Blocked { .. } => Unreached::Refused,
                _ => Unreached::Failed,
            })?
            .into_parts();
        let mut answer = Answer::new(parts.status.as_u16(), body);
        for (name, value) in &parts.headers {
            if let Ok(value) = value.to_str() {
                let value = match path_on(value, &self.asked) {
                    Some(rest) if name == http::header::LOCATION => format!("{}{rest}", self.site),
                    _ => value.to_owned(),
                };
                answer = answer.with_header(name.as_str(), &value);
            }
        }
        Ok(answer)
    }
}

/// The path of `url` when it is on `origin` exactly, starting with `/`.
fn path_on<'a>(url: &'a str, origin: &str) -> Option<&'a str> {
    url.strip_prefix(origin)
        .filter(|path| path.starts_with('/'))
}
