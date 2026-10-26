//! The `Khalid-Signature` header: signing a delivery.

use std::fmt;

use chrono::{DateTime, Utc};
use hmac::digest::InvalidLength;
use hmac::{Hmac, KeyInit, Mac};
use khalid_core::Secret;
use sha2::Sha256;

/// The header that carries the signature on every delivery.
pub const SIGNATURE_HEADER: &str = "Khalid-Signature";

/// The shortest key accepted: 256 bits, as RULES.md BL-SEC-3 asks of tokens.
const MIN_KEY_BYTES: usize = 32;

/// The length of an HMAC-SHA256 output in bytes.
pub(crate) const MAC_BYTES: usize = 32;

/// HMAC with SHA-256, the one MAC webhooks use.
pub(crate) type HmacSha256 = Hmac<Sha256>;

/// The secret a webhook shares with its receiver, ready to sign with.
///
/// The key is the UTF-8 bytes of the secret as the receiver sees it, so a
/// receiver in any language can check a delivery with its own HMAC library.
/// `Debug` never shows it.
#[derive(Clone)]
pub struct SigningKey(HmacSha256);

impl SigningKey {
    /// Prepares `secret` for signing.
    ///
    /// # Errors
    ///
    /// Returns [`SigningKeyError::TooShort`] when the secret has fewer than 32
    /// bytes.
    pub fn try_new(secret: &Secret<String>) -> Result<Self, SigningKeyError> {
        let bytes = secret.expose().as_bytes();
        if bytes.len() < MIN_KEY_BYTES {
            return Err(SigningKeyError::TooShort { len: bytes.len() });
        }
        HmacSha256::new_from_slice(bytes)
            .map(Self)
            .map_err(|source| SigningKeyError::Rejected { source })
    }

    /// The MAC over `timestamp`, a full stop and `body`.
    pub(crate) fn mac(&self, timestamp: i64, body: &[u8]) -> HmacSha256 {
        let mut mac = self.0.clone();
        mac.update(timestamp.to_string().as_bytes());
        mac.update(b".");
        mac.update(body);
        mac
    }
}

impl fmt::Debug for SigningKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SigningKey([redacted])")
    }
}

/// Why a secret cannot be used as a signing key.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SigningKeyError {
    /// The secret is too short to resist guessing.
    #[error("a webhook signing key needs at least {MIN_KEY_BYTES} bytes, and this one has {len}")]
    TooShort {
        /// The length given, in bytes.
        len: usize,
    },
    /// The HMAC implementation refused the key.
    #[error("the webhook signing key was refused")]
    Rejected {
        /// What the HMAC implementation reported.
        #[source]
        source: InvalidLength,
    },
}

/// A signed timestamp and body. `Display` writes the header value,
/// `t=<unix seconds>,v1=<lowercase hex>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    timestamp: i64,
    v1: [u8; MAC_BYTES],
}

impl Signature {
    /// The signing time, in seconds since 1970 UTC.
    pub fn timestamp(&self) -> i64 {
        self.timestamp
    }
}

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "t={},v1=", self.timestamp)?;
        self.v1.iter().try_for_each(|byte| write!(f, "{byte:02x}"))
    }
}

/// Signs `body` as sent at `at`: HMAC-SHA256 with `key` over the Unix
/// timestamp, a full stop and the body (RULES.md BL-SEC-7).
///
/// # Examples
///
/// ```
/// use chrono::DateTime;
/// use khalid_core::Secret;
/// use khalid_webhooks::{SigningKey, sign};
///
/// let key = SigningKey::try_new(&Secret::new("made-up-key-for-this-example-only".to_owned()))?;
/// let at = DateTime::from_timestamp(1_793_001_600, 0).ok_or("no such time")?;
/// let header = sign(&key, at, b"{}").to_string();
/// assert!(header.starts_with("t=1793001600,v1="));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn sign(key: &SigningKey, at: DateTime<Utc>, body: &[u8]) -> Signature {
    let timestamp = at.timestamp();
    let v1 = key.mac(timestamp, body).finalize().into_bytes().into();
    Signature { timestamp, v1 }
}
