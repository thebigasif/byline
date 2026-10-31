//! [`BingWebmaster`]: the Bing Webmaster Tools API, reached through
//! [`Outbound`] with the site owner's API key.

use khalid_core::{AbsoluteUrl, Secret};
use khalid_outbound::{Fetched, Outbound, OutboundError};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::json;

use crate::{BingError, BingSitemap, CrawlDay, CrawlIssue, RequestFailure};

/// The API's JSON endpoint; each method's name follows it.
const ENDPOINT: &str = "https://ssl.bing.com/webmaster/api.svc/json/";

/// The header a request with a body carries.
const JSON: (&str, &str) = ("Content-Type", "application/json; charset=utf-8");

/// The Bing Webmaster Tools API for one site that its owner verified there.
///
/// The key travels in each request's address, as the API asks, so neither
/// this type nor its errors ever print an address that holds it: `Debug`
/// shows the key as `[redacted]`, and a failed request keeps only the kind
/// of failure (RULES.md BL-RUN-5).
#[derive(Debug)]
pub struct BingWebmaster {
    endpoint: String,
    site: AbsoluteUrl,
    key: Secret<String>,
}

impl BingWebmaster {
    /// The API for `site`, the address verified in Bing Webmaster Tools,
    /// with the owner's API `key`.
    ///
    /// # Errors
    ///
    /// [`BingError::Key`] when the key is empty or holds anything but ASCII
    /// letters and digits, which Bing's keys never do.
    pub fn new(site: AbsoluteUrl, key: Secret<String>) -> Result<Self, BingError> {
        let text = key.expose();
        if text.is_empty() || !text.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(BingError::Key);
        }
        Ok(Self {
            endpoint: ENDPOINT.to_owned(),
            site,
            key,
        })
    }

    /// The same API reached at `endpoint` instead of Bing's, such as a
    /// stand-in server in a test. Method names are added to it as they are.
    #[must_use]
    pub fn with_endpoint(self, endpoint: &AbsoluteUrl) -> Self {
        Self {
            endpoint: endpoint.as_str().to_owned(),
            ..self
        }
    }

    /// Hands Bing the sitemap at `sitemap`, so it reads the site's pages
    /// from it (`SubmitFeed`).
    ///
    /// # Errors
    ///
    /// The errors [`BingWebmaster::crawl_stats`] returns.
    pub async fn submit_sitemap(
        &self,
        outbound: &Outbound,
        sitemap: &AbsoluteUrl,
    ) -> Result<(), BingError> {
        let body = json!({ "siteUrl": self.site.as_str(), "feedUrl": sitemap.as_str() });
        let url = self.address("SubmitFeed", None)?;
        let answer = outbound
            .post(&url, &[JSON], body.to_string().into_bytes())
            .await;
        read(answer).map(drop)
    }

    /// The sitemaps and feeds Bing holds for the site, with how its last
    /// read of each went (`GetFeeds`).
    ///
    /// # Errors
    ///
    /// The errors [`BingWebmaster::crawl_stats`] returns.
    pub async fn sitemaps(&self, outbound: &Outbound) -> Result<Vec<BingSitemap>, BingError> {
        self.get(outbound, "GetFeeds").await
    }

    /// Bing's crawl of the site, one entry a day for up to six months
    /// (`GetCrawlStats`).
    ///
    /// # Errors
    ///
    /// [`BingError::KeyRefused`] when Bing does not know the key,
    /// [`BingError::NotVerified`] when the key's owner has not verified the
    /// site, [`BingError::Refused`] for any other refusal,
    /// [`BingError::Request`] when the request did not complete, and
    /// [`BingError::Answer`] when the answer is not the API's JSON.
    pub async fn crawl_stats(&self, outbound: &Outbound) -> Result<Vec<CrawlDay>, BingError> {
        self.get(outbound, "GetCrawlStats").await
    }

    /// The addresses Bing had trouble crawling (`GetCrawlIssues`). A fixed
    /// one may stay listed for a few days.
    ///
    /// # Errors
    ///
    /// The errors [`BingWebmaster::crawl_stats`] returns.
    pub async fn crawl_issues(&self, outbound: &Outbound) -> Result<Vec<CrawlIssue>, BingError> {
        self.get(outbound, "GetCrawlIssues").await
    }

    /// Calls the GET method `name` for the site and reads its list.
    async fn get<T: DeserializeOwned>(
        &self,
        outbound: &Outbound,
        name: &str,
    ) -> Result<Vec<T>, BingError> {
        let url = self.address(name, Some(&self.site))?;
        let body = read(outbound.get(&url).await)?;
        let wrapped: Wrapped<Vec<T>> =
            serde_json::from_slice(&body).map_err(|_| BingError::Answer)?;
        Ok(wrapped.d)
    }

    /// The address of method `name`, with `site` as `siteUrl` when given,
    /// then the key. It stays in the caller's frame and is never logged.
    fn address(&self, name: &str, site: Option<&AbsoluteUrl>) -> Result<AbsoluteUrl, BingError> {
        let mut text = format!("{}{name}?", self.endpoint);
        if let Some(site) = site {
            text.push_str("siteUrl=");
            encode(site.as_str(), &mut text);
            text.push('&');
        }
        text.push_str("apikey=");
        text.push_str(self.key.expose());
        AbsoluteUrl::parse(&text).map_err(|_| BingError::Address)
    }
}

/// The `{"d": ...}` wrapper every JSON answer comes in.
#[derive(Debug, Deserialize)]
struct Wrapped<T> {
    d: T,
}

/// The body of a 2xx answer, or why there was none.
fn read(answer: Result<Fetched, OutboundError>) -> Result<Vec<u8>, BingError> {
    let fetched = answer.map_err(|error| BingError::Request {
        failure: RequestFailure::from(&error),
    })?;
    let status = fetched.status();
    if (200..300).contains(&status) {
        return Ok(fetched.body().to_vec());
    }
    // A refusal reads {"ErrorCode": 3, "Message": "InvalidApiKey"}.
    let fault: serde_json::Value = serde_json::from_slice(fetched.body()).unwrap_or_default();
    let message = fault.get("Message").and_then(|m| m.as_str());
    Err(BingError::refused(status, message.unwrap_or_default()))
}

/// Appends `text` to `out` with every byte but letters, digits and
/// `-._~` written as `%XX`, as a query value needs.
fn encode(text: &str, out: &mut String) {
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(byte));
        } else {
            out.push('%');
            for nibble in [byte >> 4, byte & 0x0F] {
                if let Some(digit) = char::from_digit(u32::from(nibble), 16) {
                    out.push(digit.to_ascii_uppercase());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_a_site_address_for_the_query() {
        let mut out = String::new();
        encode("https://www.example.com/blog/?a=b&c", &mut out);
        assert_eq!(out, "https%3A%2F%2Fwww.example.com%2Fblog%2F%3Fa%3Db%26c");
    }
}
