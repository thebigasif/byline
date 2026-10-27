//! The name every derived file shares: a hash of the original's bytes.

use std::fmt;
use std::fmt::Write as _;

use sha2::{Digest, Sha256};

/// How many hex digits of SHA-256 name an original. Sixteen digits, 64
/// bits, make a collision between two images of one site unthinkable while
/// keeping file names short.
const KEY_DIGITS: usize = 16;

/// A hash of an original image's bytes. Two identical files get the same
/// key, so they share derived files; a changed file gets a new key, so no
/// stale size is ever served.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentKey(String);

impl ContentKey {
    /// The key of `original`.
    pub fn of(original: &[u8]) -> Self {
        let hash = Sha256::digest(original);
        let mut key = String::with_capacity(KEY_DIGITS);
        for byte in hash.iter().take(KEY_DIGITS / 2) {
            let _ = write!(key, "{byte:02x}");
        }
        Self(key)
    }

    /// Reads a key written by [`ContentKey::as_str`], such as a media item's
    /// identity in an address, or `None` unless it is 16 lowercase hex
    /// digits, so a key can always be used as a folder name.
    pub fn parse(text: &str) -> Option<Self> {
        let hex = |c: char| c.is_ascii_digit() || ('a'..='f').contains(&c);
        (text.len() == KEY_DIGITS && text.chars().all(hex)).then(|| Self(text.to_owned()))
    }

    /// The key as lowercase hex, such as `3f2a9c1e0b57d0aa`.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ContentKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
