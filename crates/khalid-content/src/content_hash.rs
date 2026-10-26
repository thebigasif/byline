//! A fingerprint of everything an entry's derived output depends on.

use std::fmt;

use sha2::{Digest, Sha256};

/// The SHA-256 of an entry's text and, for a folder post, every asset beside
/// it. Derived output (rendered pages, resized images, indexes) is keyed by
/// this, so changing one image re-renders that post and nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentHash([u8; 32]);

impl ContentHash {
    /// Hashes `text`, then each `(name, bytes)` asset in the order given, with
    /// every part length-prefixed so no two inputs can run together.
    pub(crate) fn of<'a>(
        text: &str,
        assets: impl IntoIterator<Item = (&'a str, &'a [u8])>,
    ) -> Self {
        let mut hasher = Sha256::new();
        part(&mut hasher, text.as_bytes());
        for (name, bytes) in assets {
            part(&mut hasher, name.as_bytes());
            part(&mut hasher, bytes);
        }
        Self(hasher.finalize().into())
    }
}

fn part(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update(u64::try_from(bytes.len()).unwrap_or(u64::MAX).to_le_bytes());
    hasher.update(bytes);
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.iter().try_for_each(|byte| write!(f, "{byte:02x}"))
    }
}
