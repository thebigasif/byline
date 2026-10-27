//! The domain name a certificate is for.

use std::fmt;
use std::str::FromStr;

/// The longest domain name DNS allows, in characters.
const MAX_LENGTH: usize = 253;

/// The longest label, the part between two dots, DNS allows.
const MAX_LABEL: usize = 63;

/// A domain name such as `blog.example.com`: lower case, at least two
/// labels, each of letters, digits and inner hyphens. It is safe to put in
/// a header or a file name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Domain(String);

impl Domain {
    /// Checks `text` and keeps it in lower case.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] when `text` is not a domain name a public
    /// certificate can be issued for, such as an address, a single word or
    /// a name with a space.
    pub fn try_new(text: &str) -> Result<Self, DomainError> {
        let name = text.trim_end_matches('.').to_ascii_lowercase();
        let labels: Vec<&str> = name.split('.').collect();
        let fits = !name.is_empty() && name.len() <= MAX_LENGTH && labels.len() >= 2;
        let labels_fit = labels.iter().all(|label| {
            !label.is_empty()
                && label.len() <= MAX_LABEL
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        });
        let last_is_word = labels
            .last()
            .is_some_and(|last| last.chars().any(|c| c.is_ascii_lowercase()));
        if fits && labels_fit && last_is_word {
            Ok(Self(name))
        } else {
            Err(DomainError {
                given: text.to_owned(),
            })
        }
    }

    /// The name, such as `blog.example.com`.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Domain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for Domain {
    type Err = DomainError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::try_new(text)
    }
}

/// The text given is not a domain name a certificate can be issued for.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{given} is not a domain name such as blog.example.com")]
pub struct DomainError {
    given: String,
}
