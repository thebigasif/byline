//! A key as the store keeps it: never the secret, only its hash.

use argon2::{Argon2, PasswordHasher, PasswordVerifier};
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
        let params = argon2::Params::new(
            argon2::Params::MIN_M_COST,
            argon2::Params::MIN_T_COST,
            1,
            None,
        )
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
        let same_secret = self.same_secret(token.secret().expose().as_bytes());
        same_secret && self.id == *token.id() && self.kind() == token.kind()
    }

    /// Whether `secret` hashes to the stored hash. Both ways compare in
    /// constant time: SHA-256 through [`same_digest`], and Argon2id inside
    /// `verify_password`, which compares its output with `subtle` too.
    fn same_secret(&self, secret: &[u8]) -> bool {
        match &self.hash {
            KeyHash::Sha256(stored) => same_digest(stored, &Sha256::digest(secret).into()),
            KeyHash::Argon2id(phc) => Argon2::default()
                .verify_password(secret, phc.as_str())
                .is_ok(),
        }
    }

    fn kind(&self) -> KeyKind {
        match self.access {
            Access::Content => KeyKind::Content,
            Access::Integration(_) => KeyKind::Integration,
            Access::Staff(_) => KeyKind::Staff,
        }
    }
}

/// Whether two digests are equal, in a time that does not depend on the
/// first byte where they differ (RULES.md BL-SEC-3).
fn same_digest(stored: &[u8; 32], presented: &[u8; 32]) -> bool {
    bool::from(stored.ct_eq(presented))
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A secret made up for these tests. It opens nothing.
    const SECRET: &str = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";

    fn token(prefix: &str, secret: &str) -> KeyToken {
        format!("{prefix}_0123456789abcdef_{secret}")
            .parse()
            .unwrap()
    }

    /// `SECRET` with its byte at `at` changed: a wrong key of the right length.
    fn wrong_at(at: usize) -> String {
        let mut wrong = SECRET.as_bytes().to_vec();
        wrong[at] = if wrong[at] == b'0' { b'1' } else { b'0' };
        String::from_utf8(wrong).unwrap()
    }

    #[test]
    fn keys_that_can_write_are_stored_as_argon2id_with_the_owasp_settings() {
        let hashing = KeyHashing::new();
        let integration = token("ki", SECRET);
        let staff = token("ks", SECRET);
        let stored = [
            StoredKey::for_integration(&integration, vec![Scope::PostsWrite], &hashing).unwrap(),
            StoredKey::for_staff(&staff, Role::Editor, &hashing).unwrap(),
        ];
        for key in stored {
            let KeyHash::Argon2id(phc) = key.hash() else {
                panic!("{:?} is not Argon2id", key.hash());
            };
            assert!(phc.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"), "{phc}");
            assert!(!phc.contains(SECRET), "the secret is never stored");
        }
        let content = StoredKey::for_token(&token("kc", SECRET));
        assert!(matches!(content.hash(), KeyHash::Sha256(_)));
    }

    #[test]
    fn a_wrong_content_key_of_the_right_length_is_compared_in_constant_time() {
        let stored_key = StoredKey::for_token(&token("kc", SECRET));
        let KeyHash::Sha256(stored) = stored_key.hash() else {
            panic!("a content key is SHA-256");
        };
        for at in [0, 31, SECRET.len() - 1] {
            let wrong = wrong_at(at);
            assert_eq!(wrong.len(), SECRET.len());
            let presented = Sha256::digest(wrong.as_bytes()).into();
            assert!(!same_digest(stored, &presented), "byte {at}");
            assert!(!stored_key.matches(&token("kc", &wrong)), "byte {at}");
        }
        let right = Sha256::digest(SECRET.as_bytes()).into();
        assert!(same_digest(stored, &right));
    }

    #[test]
    fn a_wrong_integration_key_of_the_right_length_is_refused() {
        let hashing = KeyHashing::fast_for_tests();
        let right = token("ki", SECRET);
        let stored = StoredKey::for_integration(&right, vec![Scope::AdminAll], &hashing).unwrap();
        assert!(stored.matches(&right));
        for at in [0, SECRET.len() - 1] {
            assert!(!stored.matches(&token("ki", &wrong_at(at))), "byte {at}");
        }
    }
}
