//! A fingerprint of everything an entry's derived output depends on.

use std::fmt;

use sha2::{Digest, Sha256};

/// The SHA-256 of an entry's text. Derived output (rendered pages, indexes)
/// is keyed by this, so a change to one post re-renders that post and
/// nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentHash([u8; 32]);

impl ContentHash {
    /// Hashes `text`.
    pub(crate) fn of(text: &str) -> Self {
        Self(Sha256::digest(text.as_bytes()).into())
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.iter().try_for_each(|byte| write!(f, "{byte:02x}"))
    }
}
