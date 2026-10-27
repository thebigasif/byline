//! Passwords: the rules a new one meets, and Argon2id to store and check it
//! (RULES.md BL-SEC-3).

use std::sync::{Arc, OnceLock};

use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use khalid_core::Secret;

/// The fewest characters a password may have. NIST SP 800-63B asks for at
/// least 8; 12 keeps a guessed password out of reach of an offline attack
/// on a stolen database for years at Argon2id's cost.
pub const MIN_CHARS: usize = 12;

/// The most bytes a password may have, so nobody makes Khalid hash a
/// megabyte on every attempt. A passphrase of 200 words still fits.
pub const MAX_BYTES: usize = 1024;

/// A password someone typed. `Debug` never shows it (BL-RUN-5).
#[derive(Debug)]
pub struct Password(Secret<String>);

impl Password {
    /// A password as typed, checked against [`MIN_CHARS`] and
    /// [`MAX_BYTES`]. Nothing else is required: length is what makes a
    /// password hard to guess.
    ///
    /// # Errors
    ///
    /// Returns [`WeakPasswordError`] when it is too short or too long.
    pub fn new(text: String) -> Result<Self, WeakPasswordError> {
        if text.chars().count() < MIN_CHARS {
            return Err(WeakPasswordError::TooShort);
        }
        if text.len() > MAX_BYTES {
            return Err(WeakPasswordError::TooLong);
        }
        Ok(Self(Secret::new(text)))
    }

    fn bytes(&self) -> &[u8] {
        self.0.expose().as_bytes()
    }
}

/// A password that does not meet the rules for a new one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum WeakPasswordError {
    /// Fewer than [`MIN_CHARS`] characters.
    #[error("a password needs at least 12 characters")]
    TooShort,
    /// More than [`MAX_BYTES`] bytes.
    #[error("a password may have at most 1024 bytes")]
    TooLong,
}

/// A stored password: an Argon2id hash in PHC form, never the password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PasswordHash(String);

impl PasswordHash {
    /// A hash read back from the store.
    ///
    /// # Errors
    ///
    /// Returns [`PasswordError::Unreadable`] when it is not an Argon2id
    /// hash in PHC form.
    pub fn from_phc(phc: String) -> Result<Self, PasswordError> {
        if phc.starts_with("$argon2id$") {
            Ok(Self(phc))
        } else {
            Err(PasswordError::Unreadable)
        }
    }

    /// The PHC string, for a store to write.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Argon2id with the settings new hashes use. Checking a hash uses the
/// settings stored in it, so changing these never locks anyone out.
/// Cloning shares the dummy hash.
#[derive(Debug, Clone)]
pub struct PasswordHashing {
    argon2: Argon2<'static>,
    /// A hash of nothing anyone knows, checked when an email has no
    /// password, so that answer takes as long as a wrong password.
    dummy: Arc<OnceLock<String>>,
}

impl PasswordHashing {
    /// Argon2id with the settings OWASP recommends: 19 MiB, two passes.
    #[must_use]
    pub fn new() -> Self {
        Self::with(Argon2::default())
    }

    /// Argon2id at the lowest cost, so tests stay fast. Never for real
    /// passwords.
    ///
    /// # Panics
    ///
    /// Never: the parameters are the library's documented minimums.
    #[cfg(any(test, feature = "test-util"))]
    #[must_use]
    #[expect(clippy::expect_used, reason = "the minimums are always valid")]
    pub fn fast_for_tests() -> Self {
        let params = argon2::Params::new(
            argon2::Params::MIN_M_COST,
            argon2::Params::MIN_T_COST,
            1,
            None,
        )
        .expect("the minimum Argon2 parameters are valid");
        Self::with(Argon2::from(params))
    }

    fn with(argon2: Argon2<'static>) -> Self {
        Self {
            argon2,
            dummy: Arc::new(OnceLock::new()),
        }
    }

    /// Hashes `password` on a blocking thread, because Argon2id is slow on
    /// purpose (BL-RUN-7).
    ///
    /// # Errors
    ///
    /// Returns [`PasswordError`] when Argon2id fails or its thread stops.
    pub async fn hash(&self, password: Password) -> Result<PasswordHash, PasswordError> {
        let argon2 = self.argon2.clone();
        let phc = tokio::task::spawn_blocking(move || hash_now(&argon2, password.bytes()))
            .await
            .map_err(|_| PasswordError::Stopped)??;
        Ok(PasswordHash(phc))
    }

    /// Whether `password` matches `stored`, on a blocking thread. With no
    /// stored hash it checks a dummy and answers `false`, in the same time.
    ///
    /// # Errors
    ///
    /// Returns [`PasswordError`] when the stored hash cannot be read or the
    /// thread stops.
    pub async fn verify(
        &self,
        password: Password,
        stored: Option<PasswordHash>,
    ) -> Result<bool, PasswordError> {
        let argon2 = self.argon2.clone();
        let dummy = Arc::clone(&self.dummy);
        tokio::task::spawn_blocking(move || {
            let (phc, real) = match stored {
                Some(PasswordHash(phc)) => (phc, true),
                None => (dummy_hash(&argon2, &dummy)?, false),
            };
            let parsed = argon2::PasswordHash::new(&phc).map_err(|_| PasswordError::Unreadable)?;
            Ok(real && argon2.verify_password(password.bytes(), &parsed).is_ok())
        })
        .await
        .map_err(|_| PasswordError::Stopped)?
    }
}

impl Default for PasswordHashing {
    fn default() -> Self {
        Self::new()
    }
}

fn hash_now(argon2: &Argon2<'static>, secret: &[u8]) -> Result<String, PasswordError> {
    argon2
        .hash_password(secret)
        .map(|hash| hash.to_string())
        .map_err(|source| PasswordError::Hash { source })
}

fn dummy_hash(argon2: &Argon2<'static>, dummy: &OnceLock<String>) -> Result<String, PasswordError> {
    if let Some(phc) = dummy.get() {
        return Ok(phc.clone());
    }
    let phc = hash_now(argon2, b"no account has this password")?;
    Ok(dummy.get_or_init(|| phc).clone())
}

/// A password could not be hashed or checked.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PasswordError {
    /// Argon2id reported an error.
    #[error("the password could not be hashed")]
    Hash {
        /// What Argon2id reported.
        #[source]
        source: argon2::password_hash::Error,
    },
    /// A stored hash is not an Argon2id hash in PHC form.
    #[error("the stored password hash cannot be read")]
    Unreadable,
    /// The thread doing the work stopped before it finished.
    #[error("the password check stopped before it finished")]
    Stopped,
}
