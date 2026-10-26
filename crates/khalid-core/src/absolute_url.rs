//! A full web address, checked once.

use std::fmt;
use std::str::FromStr;

/// An `http` or `https` address with a host and no credentials in it.
///
/// Site addresses, canonical links and the pages theme sync reads all take
/// this type, so code that receives one never parses or checks it again
/// (RULES.md BL-TYP-3). The text keeps the WHATWG standard form.
///
/// # Examples
///
/// ```
/// use khalid_core::AbsoluteUrl;
///
/// let site = AbsoluteUrl::parse("https://Example.com")?;
/// assert_eq!(site.as_str(), "https://example.com/");
/// assert_eq!(site.host(), "example.com");
/// # Ok::<(), khalid_core::ParseUrlError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AbsoluteUrl(url::Url);

impl AbsoluteUrl {
    /// Parses `text` and checks the rules above.
    ///
    /// # Errors
    ///
    /// Returns a [`ParseUrlError`] when `text` is not a URL, uses a scheme
    /// other than `http` or `https`, has no host, or carries a user name or
    /// password. The error never repeats the credentials.
    pub fn parse(text: &str) -> Result<Self, ParseUrlError> {
        let url = url::Url::parse(text).map_err(|source| ParseUrlError::Invalid { source })?;
        if !matches!(url.scheme(), "http" | "https") {
            return Err(ParseUrlError::Scheme {
                scheme: url.scheme().to_owned(),
            });
        }
        if !url.username().is_empty() || url.password().is_some() {
            return Err(ParseUrlError::Credentials);
        }
        if url.host_str().is_none() {
            return Err(ParseUrlError::NoHost);
        }
        Ok(Self(url))
    }

    /// The address as text, with a lowercase scheme and host.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// The host name, such as `www.example.com`.
    pub fn host(&self) -> &str {
        self.0.host_str().unwrap_or_default()
    }
}

impl FromStr for AbsoluteUrl {
    type Err = ParseUrlError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::parse(text)
    }
}

impl fmt::Display for AbsoluteUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Why a text is not an [`AbsoluteUrl`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ParseUrlError {
    /// The text is not a URL at all.
    #[error("the address is not a full URL starting with https://")]
    Invalid {
        /// What the URL parser found.
        #[source]
        source: url::ParseError,
    },
    /// The scheme is not `http` or `https`.
    #[error("the address uses {scheme}:, and only http: and https: are allowed")]
    Scheme {
        /// The scheme the text used.
        scheme: String,
    },
    /// The URL has no host.
    #[error("the address has no host name")]
    NoHost,
    /// The URL carries a user name or a password.
    #[error("the address contains a user name or password, which must never be stored in it")]
    Credentials,
}
