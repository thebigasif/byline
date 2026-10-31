//! [`SearchConsole`] and [`Session`]: signing in to Google with the
//! owner's refresh token, and the requests a signed-in session sends.

use std::fmt;

use khalid_core::{AbsoluteUrl, SearchConsoleProperty, Secret};
use khalid_outbound::{Fetched, Outbound, OutboundError};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::json;

use crate::{IndexStatus, PageNumbers, SearchConsoleError, Window};

/// Where Google trades a refresh token for an access token.
const TOKEN: &str = "https://oauth2.googleapis.com/token";

/// The Search Console API's root (`searchAnalytics` and `sites`).
const WEBMASTERS: &str = "https://www.googleapis.com/webmasters/v3/";

/// The URL Inspection API's root.
const INSPECTION: &str = "https://searchconsole.googleapis.com/v1/";

/// The most rows one Search Analytics answer holds.
const MAX_ROWS: u32 = 25_000;

/// The OAuth client the owner made in Google Cloud, and the refresh token
/// Google gave when the owner let it read their Search Console numbers.
#[derive(Debug)]
pub struct Credentials {
    /// The client's identity, such as `1234-abcd.apps.googleusercontent.com`.
    pub client_id: String,
    /// The client's secret.
    pub client_secret: Secret<String>,
    /// The refresh token.
    pub refresh_token: Secret<String>,
}

/// One Search Console property and the credentials that read it. `Debug`
/// hides both secrets.
#[derive(Debug)]
pub struct SearchConsole {
    property: String,
    credentials: Credentials,
    token: String,
    webmasters: String,
    inspection: String,
}

impl SearchConsole {
    /// The property `property`, read with `credentials`.
    pub fn new(property: &SearchConsoleProperty, credentials: Credentials) -> Self {
        Self {
            property: property.as_str(),
            credentials,
            token: TOKEN.to_owned(),
            webmasters: WEBMASTERS.to_owned(),
            inspection: INSPECTION.to_owned(),
        }
    }

    /// The same connection with every Google address under `endpoint`
    /// instead, such as a stand-in server in a test: `token`,
    /// `webmasters/v3/` and `v1/`.
    #[must_use]
    pub fn with_endpoint(self, endpoint: &AbsoluteUrl) -> Self {
        let root = endpoint.as_str().trim_end_matches('/');
        Self {
            token: format!("{root}/token"),
            webmasters: format!("{root}/webmasters/v3/"),
            inspection: format!("{root}/v1/"),
            ..self
        }
    }

    /// The property, as Google names it.
    pub fn property(&self) -> &str {
        &self.property
    }

    /// Trades the refresh token for an access token, which works for about
    /// an hour, and opens a session with it.
    ///
    /// # Errors
    ///
    /// [`SearchConsoleError::TokenRefused`] when Google no longer accepts
    /// the refresh token, [`SearchConsoleError::ClientRefused`] when it
    /// does not know the client, [`SearchConsoleError::Request`] when the
    /// request did not complete, and [`SearchConsoleError::Answer`] when
    /// the answer is not OAuth's JSON.
    pub async fn sign_in<'a>(
        &'a self,
        outbound: &'a Outbound,
    ) -> Result<Session<'a>, SearchConsoleError> {
        let credentials = &self.credentials;
        let mut form = String::from("grant_type=refresh_token");
        for (name, value) in [
            ("client_id", credentials.client_id.as_str()),
            ("client_secret", credentials.client_secret.expose()),
            ("refresh_token", credentials.refresh_token.expose()),
        ] {
            form.push('&');
            form.push_str(name);
            form.push('=');
            encode(value, &mut form);
        }
        let url = address(&self.token)?;
        let form_type = ("Content-Type", "application/x-www-form-urlencoded");
        let answer = outbound.post(&url, &[form_type], form.into_bytes()).await;
        let granted: Granted = parse(&read(answer)?)?;
        Ok(Session {
            console: self,
            outbound,
            bearer: Secret::new(format!("Bearer {}", granted.access_token)),
        })
    }
}

/// OAuth's answer to a refresh.
#[derive(Deserialize)]
struct Granted {
    access_token: String,
}

/// A signed-in connection, good for about an hour. `Debug` hides its
/// access token.
pub struct Session<'a> {
    console: &'a SearchConsole,
    outbound: &'a Outbound,
    bearer: Secret<String>,
}

impl fmt::Debug for Session<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Session")
            .field("property", &self.console.property)
            .finish_non_exhaustive()
    }
}

impl Session<'_> {
    /// What Google counted for each page of the property in `window`, the
    /// most clicked first. Google holds about 16 months, and its last two
    /// or three days are not final yet.
    ///
    /// # Errors
    ///
    /// [`SearchConsoleError::NotAllowed`] when the signed-in account may
    /// not read the property, [`SearchConsoleError::Quota`] when Google's
    /// quota is spent, and otherwise the errors of
    /// [`SearchConsole::sign_in`].
    pub async fn pages(&self, window: Window) -> Result<Vec<PageNumbers>, SearchConsoleError> {
        let body = json!({
            "startDate": window.first.to_string(),
            "endDate": window.last.to_string(),
            "dimensions": ["page"],
            "rowLimit": MAX_ROWS,
        });
        PageNumbers::read_all(&self.call(&self.analytics()?, &body).await?)
    }

    /// The searches that showed `page` in `window`, up to `limit`, the
    /// most clicked first. Each row's key is the query.
    ///
    /// # Errors
    ///
    /// The errors of [`Session::pages`].
    pub async fn queries(
        &self,
        window: Window,
        page: &str,
        limit: u32,
    ) -> Result<Vec<PageNumbers>, SearchConsoleError> {
        let only = json!({ "dimension": "page", "operator": "equals", "expression": page });
        let body = json!({
            "startDate": window.first.to_string(),
            "endDate": window.last.to_string(),
            "dimensions": ["query"],
            "dimensionFilterGroups": [{ "filters": [only] }],
            "rowLimit": limit.min(MAX_ROWS),
        });
        PageNumbers::read_all(&self.call(&self.analytics()?, &body).await?)
    }

    /// Whether Google has `page` in its index. Google allows 2,000 of
    /// these a day and 600 a minute for each property.
    ///
    /// # Errors
    ///
    /// [`SearchConsoleError::Quota`] when the day's or the minute's calls
    /// are spent, and otherwise the errors of [`Session::pages`].
    pub async fn inspect(&self, page: &AbsoluteUrl) -> Result<IndexStatus, SearchConsoleError> {
        let body = json!({ "inspectionUrl": page.as_str(), "siteUrl": self.console.property });
        let url = address(&format!(
            "{}urlInspection/index:inspect",
            self.console.inspection
        ))?;
        IndexStatus::read(&self.call(&url, &body).await?)
    }

    /// The Search Analytics address of the property.
    fn analytics(&self) -> Result<AbsoluteUrl, SearchConsoleError> {
        let mut text = format!("{}sites/", self.console.webmasters);
        encode(&self.console.property, &mut text);
        text.push_str("/searchAnalytics/query");
        address(&text)
    }

    /// Sends `body` as JSON to `url` and hands back the answer's body.
    async fn call(
        &self,
        url: &AbsoluteUrl,
        body: &serde_json::Value,
    ) -> Result<Vec<u8>, SearchConsoleError> {
        let headers = [
            ("Authorization", self.bearer.expose().as_str()),
            ("Content-Type", "application/json"),
        ];
        let answer = self
            .outbound
            .post(url, &headers, body.to_string().into_bytes())
            .await;
        read(answer)
    }
}

/// `text` as an address, or why a request cannot be made of it.
fn address(text: &str) -> Result<AbsoluteUrl, SearchConsoleError> {
    AbsoluteUrl::parse(text).map_err(|_| SearchConsoleError::Address)
}

/// The body of a 2xx answer, or why there was none.
fn read(answer: Result<Fetched, OutboundError>) -> Result<Vec<u8>, SearchConsoleError> {
    let fetched = answer.map_err(|source| SearchConsoleError::Request { source })?;
    let status = fetched.status();
    if (200..300).contains(&status) {
        return Ok(fetched.body().to_vec());
    }
    Err(SearchConsoleError::refused(status, fetched.body()))
}

/// `body` read as `T`.
fn parse<T: DeserializeOwned>(body: &[u8]) -> Result<T, SearchConsoleError> {
    serde_json::from_slice(body).map_err(|_| SearchConsoleError::Answer)
}

/// Appends `text` to `out` with every byte but letters, digits and
/// `-._~` written as `%XX`, as a form value or a path segment needs.
pub(crate) fn encode(text: &str, out: &mut String) {
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
    fn encodes_a_property_as_one_path_segment() {
        let mut out = String::new();
        encode("sc-domain:example.com", &mut out);
        assert_eq!(out, "sc-domain%3Aexample.com");
        out.clear();
        encode("https://www.example.com/", &mut out);
        assert_eq!(out, "https%3A%2F%2Fwww.example.com%2F");
    }
}
