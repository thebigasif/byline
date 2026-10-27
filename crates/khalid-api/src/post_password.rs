//! A post's password: taken once through the Admin API, kept only as an
//! Argon2id hash, and checked when a reader gives it.

use std::fmt;

use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use khalid_core::{PasswordHash, Secret};
use serde::{Deserialize, Deserializer};
use subtle::ConstantTimeEq;

/// A new password for a post, as an edit sends it. It is hashed before
/// the post is saved and never written, logged or sent back (BL-RUN-5).
pub struct NewPassword(Secret<String>);

impl NewPassword {
    /// The fewest characters a password may have.
    pub const MIN_CHARS: usize = 8;
    /// The most characters a password may have.
    pub const MAX_CHARS: usize = 128;

    /// `password` as a new post password.
    ///
    /// # Errors
    ///
    /// Returns [`ShortPasswordError`] unless it has
    /// [`MIN_CHARS`](Self::MIN_CHARS) to [`MAX_CHARS`](Self::MAX_CHARS)
    /// characters.
    pub fn try_new(password: String) -> Result<Self, ShortPasswordError> {
        let chars = password.chars().count();
        if (Self::MIN_CHARS..=Self::MAX_CHARS).contains(&chars) {
            Ok(Self(Secret::new(password)))
        } else {
            Err(ShortPasswordError)
        }
    }

    /// The Argon2id hash to keep in the post's front matter, with a fresh
    /// salt. Slow on purpose: call it on a blocking thread (BL-RUN-7).
    ///
    /// # Errors
    ///
    /// Returns [`HashPasswordError`] when Argon2id cannot hash it.
    pub fn hash(&self) -> Result<PasswordHash, HashPasswordError> {
        let phc = Argon2::default()
            .hash_password(self.0.expose().as_bytes())
            .map_err(HashPasswordError::Argon2)?;
        PasswordHash::try_new(&phc.to_string()).map_err(HashPasswordError::NotPhc)
    }
}

impl fmt::Debug for NewPassword {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("NewPassword([redacted])")
    }
}

impl Clone for NewPassword {
    fn clone(&self) -> Self {
        Self(Secret::new(self.0.expose().clone()))
    }
}

/// Compared in constant time, as every secret is (BL-SEC-3).
impl PartialEq for NewPassword {
    fn eq(&self, other: &Self) -> bool {
        let (mine, theirs) = (self.0.expose().as_bytes(), other.0.expose().as_bytes());
        mine.ct_eq(theirs).into()
    }
}

impl Eq for NewPassword {}

impl<'de> Deserialize<'de> for NewPassword {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::try_new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

/// Whether `attempt` is the password `hash` was made from. Slow on
/// purpose: call it on a blocking thread (BL-RUN-7). A hash that cannot be
/// read matches nothing, so a broken file locks the post rather than
/// opening it.
pub fn password_matches(hash: &PasswordHash, attempt: &str) -> bool {
    Argon2::default()
        .verify_password(attempt.as_bytes(), hash.as_str())
        .is_ok()
}

/// A password that is too short or too long.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("a post password needs 8 to 128 characters")]
pub struct ShortPasswordError;

/// Argon2id could not hash a new post password.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum HashPasswordError {
    /// Argon2id failed.
    #[error("the new password could not be hashed")]
    Argon2(#[source] argon2::password_hash::Error),
    /// Argon2id gave a hash that is not in the form front matter keeps.
    #[error("the new password's hash is not an Argon2id PHC string")]
    NotPhc(#[source] khalid_core::ParsePasswordHashError),
}
