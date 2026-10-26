//! A key as the store keeps it: never the secret, only its hash.

use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use super::{KeyId, KeyKind, KeyToken};

/// A key as stored: its identity, its kind and the SHA-256 of its secret.
///
/// The secret is 256 random bits, so SHA-256 cannot be reversed or guessed,
/// and it is fast enough to check on every request (ADR 0011). The
/// comparison takes the same time whichever byte differs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredKey {
    id: KeyId,
    kind: KeyKind,
    hash: [u8; 32],
}

impl StoredKey {
    /// A stored key read back from the store.
    pub fn new(id: KeyId, kind: KeyKind, hash: [u8; 32]) -> Self {
        Self { id, kind, hash }
    }

    /// What to store for `token` when it is created.
    pub fn for_token(token: &KeyToken) -> Self {
        Self {
            id: token.id().clone(),
            kind: token.kind(),
            hash: hash(token),
        }
    }

    /// The identity.
    pub fn id(&self) -> &KeyId {
        &self.id
    }

    /// Whether `token` is this key: same identity, same kind, same secret.
    pub fn matches(&self, token: &KeyToken) -> bool {
        let same_secret: bool = self.hash.ct_eq(&hash(token)).into();
        same_secret && self.id == *token.id() && self.kind == token.kind()
    }
}

fn hash(token: &KeyToken) -> [u8; 32] {
    Sha256::digest(token.secret().expose().as_bytes()).into()
}
