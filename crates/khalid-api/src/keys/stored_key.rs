//! A key as the store keeps it: never the secret, only its hash.

use argon2::{Argon2, Params, PasswordHasher, PasswordVerifier};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use super::{Access, KeyId, KeyKind, KeyToken, Role, Scope};

/// How a stored key's secret is hashed (ADR 0011).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum KeyHash {
    /// SHA-256, for content keys, which every Content API request checks.
    Sha256([u8; 32]),
    /// An Argon2id hash in PHC form, for keys that can write.
    Argon2id(String),
}

/// The Argon2id settings new hashes use. Checking a hash uses the settings
/// stored in it, so changing these never locks out an existing key.
#[derive(Debug, Clone)]
pub struct KeyHashing(Argon2<'static>);

impl KeyHashing {
    /// Argon2id with the settings OWASP recommends: 19 MiB, two passes.
    pub fn new() -> Self {
        Self(Argon2::default())
    }

    /// Argon2id at the lowest cost, so tests stay fast. Never for real keys.
    ///
    /// # Panics
    ///
    /// Never: the parameters are the library's documented minimums.
    #[cfg(any(test, feature = "test-util"))]
    #[expect(clippy::expect_used, reason = "the minimums are always valid")]
    pub fn fast_for_tests() -> Self {
        let params = Params::new(Params::MIN_M_COST, Params::MIN_T_COST, 1, None)
            .expect("the minimum Argon2 parameters are valid");
        Self(Argon2::from(params))
    }
}

impl Default for KeyHashing {
    fn default() -> Self {
        Self::new()
    }
}

/// A key as stored: its identity, what it may do, and a hash of its secret.
/// Comparisons take the same time whichever byte differs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredKey {
    id: KeyId,
    access: Access,
    hash: KeyHash,
}

impl StoredKey {
    /// A stored key read back from the store.
    pub fn new(id: KeyId, access: Access, hash: KeyHash) -> Self {
        Self { id, access, hash }
    }

    /// What to store for a new content key.
    pub fn for_token(token: &KeyToken) -> Self {
        let digest = Sha256::digest(token.secret().expose().as_bytes()).into();
        Self::new(token.id().clone(), Access::Content, KeyHash::Sha256(digest))
    }

    /// What to store for a new integration key with `scopes`.
    ///
    /// # Errors
    ///
    /// Returns [`HashKeyError`] when Argon2id cannot hash the secret.
    pub fn for_integration(
        token: &KeyToken,
        scopes: Vec<Scope>,
        hashing: &KeyHashing,
    ) -> Result<Self, HashKeyError> {
        Ok(Self::new(
            token.id().clone(),
            Access::Integration(scopes),
            argon2id(token, hashing)?,
        ))
    }

    /// What to store for a new staff token with `role`.
    ///
    /// # Errors
    ///
    /// Returns [`HashKeyError`] when Argon2id cannot hash the secret.
    pub fn for_staff(
        token: &KeyToken,
        role: Role,
        hashing: &KeyHashing,
    ) -> Result<Self, HashKeyError> {
        Ok(Self::new(
            token.id().clone(),
            Access::Staff(role),
            argon2id(token, hashing)?,
        ))
    }

    /// The identity.
    pub fn id(&self) -> &KeyId {
        &self.id
    }

    /// What the key may do.
    pub fn access(&self) -> &Access {
        &self.access
    }

    /// The hash of its secret, for a store to write.
    pub fn hash(&self) -> &KeyHash {
        &self.hash
    }

    /// Whether `token` is this key: same identity, same kind, same secret.
    pub fn matches(&self, token: &KeyToken) -> bool {
        let secret = token.secret().expose().as_bytes();
        let same_secret = match &self.hash {
            KeyHash::Sha256(stored) => {
                let digest: [u8; 32] = Sha256::digest(secret).into();
                bool::from(stored.ct_eq(&digest))
            }
            KeyHash::Argon2id(phc) => Argon2::default()
                .verify_password(secret, phc.as_str())
                .is_ok(),
        };
        same_secret && self.id == *token.id() && self.kind() == token.kind()
    }

    fn kind(&self) -> KeyKind {
        match self.access {
            Access::Content => KeyKind::Content,
            Access::Integration(_) => KeyKind::Integration,
            Access::Staff(_) => KeyKind::Staff,
        }
    }
}

fn argon2id(token: &KeyToken, hashing: &KeyHashing) -> Result<KeyHash, HashKeyError> {
    let secret = token.secret().expose().as_bytes();
    let hash = hashing
        .0
        .hash_password(secret)
        .map_err(|source| HashKeyError { source })?;
    Ok(KeyHash::Argon2id(hash.to_string()))
}

/// Argon2id could not hash a new key's secret.
#[derive(Debug, thiserror::Error)]
#[error("the new key could not be hashed")]
pub struct HashKeyError {
    #[source]
    source: argon2::password_hash::Error,
}
