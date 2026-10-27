//! The token in a confirmation or unsubscribe link.

use std::fmt;

use khalid_core::Secret;
use sha2::{Digest, Sha256};

/// How many random bytes a token holds: 256 bits (RULES.md BL-SEC-3).
const RANDOM_BYTES: usize = 32;

/// A token as it travels in a link: 64 lowercase hex digits of 256 random
/// bits. Only its [`TokenHash`] is stored, so the database never holds a
/// link that works. It is printed as `[redacted]`.
pub struct SubscriberToken(Secret<String>);

impl SubscriberToken {
    /// A new token from the operating system's randomness.
    ///
    /// # Errors
    ///
    /// [`GenerateTokenError`] when the operating system has no randomness
    /// to give.
    pub fn generate() -> Result<Self, GenerateTokenError> {
        let mut bytes = [0; RANDOM_BYTES];
        getrandom::fill(&mut bytes).map_err(|source| GenerateTokenError { source })?;
        Ok(Self(Secret::new(to_hex(&bytes))))
    }

    /// The token in `text`, as read back from a link.
    ///
    /// # Errors
    ///
    /// [`ParseTokenError`] when `text` is not 64 lowercase hex digits. A
    /// well-formed token that was never issued parses, and the store finds
    /// no hash for it.
    pub fn parse(text: &str) -> Result<Self, ParseTokenError> {
        let hex = text.len() == RANDOM_BYTES * 2
            && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
        if hex {
            Ok(Self(Secret::new(text.to_owned())))
        } else {
            Err(ParseTokenError)
        }
    }

    /// The token as text, to put in a link. Never log it.
    pub fn expose_text(&self) -> String {
        self.0.expose().clone()
    }

    /// What is stored in place of the token.
    pub fn hash(&self) -> TokenHash {
        TokenHash(Sha256::digest(self.0.expose().as_bytes()).into())
    }
}

impl fmt::Debug for SubscriberToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SubscriberToken").field(&self.0).finish()
    }
}

/// The SHA-256 of a token's text. Looking a link up by its hash leaks
/// nothing about other tokens, so no constant-time comparison is needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TokenHash([u8; 32]);

impl TokenHash {
    /// A hash read back from the store.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// The 32 bytes to store.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// `bytes` as lowercase hex digits.
fn to_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .flat_map(|byte| [byte >> 4, byte & 0x0f])
        .filter_map(|nibble| char::from_digit(u32::from(nibble), 16))
        .collect()
}

/// The operating system gave no random bytes for a new token.
#[derive(Debug, thiserror::Error)]
#[error("no random bytes for a new subscriber token")]
pub struct GenerateTokenError {
    #[source]
    source: getrandom::Error,
}

/// Text that is not a subscriber token. It does not repeat the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("this is not a subscriber token")]
pub struct ParseTokenError;
