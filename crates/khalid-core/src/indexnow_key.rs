//! [`IndexNowKey`]: the key that proves a site owns the pages it sends to
//! `IndexNow`.

use std::fmt;

/// The fewest characters `IndexNow` accepts in a key.
const MIN_LEN: usize = 8;

/// The most characters `IndexNow` accepts in a key.
const MAX_LEN: usize = 128;

/// An `IndexNow` key: 8 to 128 ASCII letters, digits and hyphens.
///
/// `IndexNow` is the shared protocol Bing, Yandex, Seznam, Naver and Yep use to
/// hear about changed pages at once. The key is public by design: the site
/// serves it in a key file so a search engine can check that the pages it is
/// told about belong to the sender. It is not a secret.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IndexNowKey(String);

impl IndexNowKey {
    /// Checks `text` against the `IndexNow` key rules.
    ///
    /// # Errors
    ///
    /// Returns [`ParseIndexNowKeyError::Length`] when `text` has fewer than 8
    /// or more than 128 characters, and [`ParseIndexNowKeyError::Character`]
    /// when it holds anything but ASCII letters, digits and hyphens.
    pub fn try_new(text: &str) -> Result<Self, ParseIndexNowKeyError> {
        if !text.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Err(ParseIndexNowKeyError::Character);
        }
        if !(MIN_LEN..=MAX_LEN).contains(&text.len()) {
            return Err(ParseIndexNowKeyError::Length);
        }
        Ok(Self(text.to_owned()))
    }

    /// The key as written.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for IndexNowKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> serde::Deserialize<'de> for IndexNowKey {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::try_new(&text).map_err(serde::de::Error::custom)
    }
}

/// Why a text is not an [`IndexNowKey`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ParseIndexNowKeyError {
    /// The key is shorter than 8 or longer than 128 characters.
    #[error(
        "the IndexNow key has the wrong length, expected a key of 8 to 128 letters, digits and hyphens"
    )]
    Length,
    /// The key holds a character other than a letter, a digit or a hyphen.
    #[error(
        "the IndexNow key holds a character that is not allowed, expected a key of 8 to 128 letters, digits and hyphens"
    )]
    Character,
}
