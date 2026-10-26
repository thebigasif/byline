//! Where keys are kept, and the check every request makes.

use std::error::Error;

use super::{KeyId, KeyKind, KeyToken, StoredKey};
use crate::content_store::BoxFuture;

/// The stored keys. `khalid-db` implements it in task 1.15;
/// `MemoryKeyStore`, from the `test-util` feature, keeps them in memory.
pub trait KeyStore: Send + Sync {
    /// The key with identity `id`, or `None`. Revoked keys are not found.
    ///
    /// # Errors
    ///
    /// Returns [`KeyStoreError`] when the keys cannot be read.
    fn find<'a>(&'a self, id: &'a KeyId)
    -> BoxFuture<'a, Result<Option<StoredKey>, KeyStoreError>>;
}

/// The keys could not be read.
#[derive(Debug, thiserror::Error)]
#[error("the keys could not be read")]
pub struct KeyStoreError {
    #[source]
    source: Box<dyn Error + Send + Sync>,
}

impl KeyStoreError {
    /// Wraps what the storage reported.
    pub fn new(source: impl Error + Send + Sync + 'static) -> Self {
        Self {
            source: Box::new(source),
        }
    }
}

/// Checks that `text` is a stored key of kind `want`, and returns its
/// identity.
///
/// # Errors
///
/// Returns [`KeyError::Rejected`] for anything that is not a stored key of
/// that kind, whatever the reason, and [`KeyError::Unavailable`] when the
/// store cannot be read.
pub async fn authenticate(
    store: &dyn KeyStore,
    text: &str,
    want: KeyKind,
) -> Result<KeyId, KeyError> {
    let token: KeyToken = text.parse().map_err(|_| KeyError::Rejected)?;
    if token.kind() != want {
        return Err(KeyError::Rejected);
    }
    let stored = store
        .find(token.id())
        .await
        .map_err(|source| KeyError::Unavailable { source })?;
    match stored {
        Some(stored) if stored.matches(&token) => Ok(token.id().clone()),
        _ => Err(KeyError::Rejected),
    }
}

/// Why a key was not accepted.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum KeyError {
    /// Not a stored key of the kind asked for. One answer for every
    /// reason, so a refusal teaches a guesser nothing.
    #[error("the key was not accepted")]
    Rejected,
    /// The keys could not be read.
    #[error("the key could not be checked")]
    Unavailable {
        /// What the store reported.
        #[source]
        source: KeyStoreError,
    },
}

#[cfg(any(test, feature = "test-util"))]
pub use memory::MemoryKeyStore;

#[cfg(any(test, feature = "test-util"))]
mod memory {
    use super::{BoxFuture, KeyId, KeyStore, KeyStoreError, StoredKey};

    /// Keys held in memory, for tests.
    #[derive(Debug, Clone, Default)]
    pub struct MemoryKeyStore {
        keys: Vec<StoredKey>,
    }

    impl MemoryKeyStore {
        /// A store holding `keys`.
        pub fn new(keys: Vec<StoredKey>) -> Self {
            Self { keys }
        }
    }

    /// Memory answers at once, so each future is ready when returned.
    impl KeyStore for MemoryKeyStore {
        fn find<'a>(
            &'a self,
            id: &'a KeyId,
        ) -> BoxFuture<'a, Result<Option<StoredKey>, KeyStoreError>> {
            let found = self.keys.iter().find(|key| key.id() == id).cloned();
            Box::pin(std::future::ready(Ok(found)))
        }
    }
}
