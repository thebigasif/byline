//! Where keys are kept, and the check every request makes.

use std::error::Error;

use super::{Access, KeyId, KeyKind, KeyToken, StoredKey};
use crate::content_store::BoxFuture;

/// The stored keys. `khalid-db` implements it on SQLite as `KeyTable`;
/// `MemoryKeyStore`, from the `test-util` feature, keeps them in memory.
pub trait KeyStore: Send + Sync {
    /// The key with identity `id`, or `None`. Revoked keys are not found.
    ///
    /// # Errors
    ///
    /// Returns [`KeyStoreError`] when the keys cannot be read.
    fn find<'a>(&'a self, id: &'a KeyId)
    -> BoxFuture<'a, Result<Option<StoredKey>, KeyStoreError>>;

    /// Stores a new key under `name`, which says who or what uses it, such
    /// as `SerpDad`. `khalid keys create` calls it once the key is hashed,
    /// so the store never sees the secret.
    ///
    /// # Errors
    ///
    /// Returns [`CreateKeyError::IdTaken`] when a key, revoked or not,
    /// already has its identity, and [`CreateKeyError::Unavailable`] when
    /// the keys cannot be written.
    fn create<'a>(
        &'a self,
        key: StoredKey,
        name: &'a str,
    ) -> BoxFuture<'a, Result<(), CreateKeyError>>;

    /// Revokes the key with identity `id`, so it is never found again.
    /// Returns whether a key that worked until now was revoked; `false`
    /// for an unknown or already revoked key.
    ///
    /// # Errors
    ///
    /// Returns [`KeyStoreError`] when the keys cannot be written.
    fn revoke<'a>(&'a self, id: &'a KeyId) -> BoxFuture<'a, Result<bool, KeyStoreError>>;
}

/// A new key could not be stored.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CreateKeyError {
    /// Another key has this identity. With 64 random bits this means the
    /// same key was stored twice.
    #[error("a key with identity {} is already stored", .0.as_str())]
    IdTaken(KeyId),
    /// The keys could not be written.
    #[error("the key could not be stored")]
    Unavailable {
        /// What the store reported.
        #[source]
        source: KeyStoreError,
    },
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

/// Who a request comes from: a stored key and what it may do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Principal {
    id: KeyId,
    access: Access,
}

impl Principal {
    /// The key's identity.
    pub fn id(&self) -> &KeyId {
        &self.id
    }

    /// What the key may do.
    pub fn access(&self) -> &Access {
        &self.access
    }
}

/// Checks that `text` is a stored key of kind `want`, and returns who it
/// belongs to and what it may do. The hash is checked on a blocking
/// thread, because Argon2id is slow on purpose (RULES.md BL-RUN-7).
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
) -> Result<Principal, KeyError> {
    let token: KeyToken = text.parse().map_err(|_| KeyError::Rejected)?;
    if token.kind() != want {
        return Err(KeyError::Rejected);
    }
    let unavailable = |source| KeyError::Unavailable { source };
    let Some(stored) = store.find(token.id()).await.map_err(unavailable)? else {
        return Err(KeyError::Rejected);
    };
    let checked = tokio::task::spawn_blocking(move || stored.matches(&token).then_some(stored));
    match checked
        .await
        .map_err(|error| unavailable(KeyStoreError::new(error)))?
    {
        Some(stored) => Ok(Principal {
            id: stored.id().clone(),
            access: stored.access().clone(),
        }),
        None => Err(KeyError::Rejected),
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
    use std::future::ready;
    use std::sync::{Mutex, MutexGuard, PoisonError};

    use super::{BoxFuture, CreateKeyError, KeyId, KeyStore, KeyStoreError, StoredKey};

    /// Keys held in memory, for tests.
    #[derive(Debug, Default)]
    pub struct MemoryKeyStore {
        keys: Mutex<Keys>,
    }

    /// The keys that work, and the identities of revoked ones, which stay
    /// taken.
    #[derive(Debug, Default)]
    struct Keys {
        working: Vec<StoredKey>,
        revoked: Vec<KeyId>,
    }

    impl MemoryKeyStore {
        /// A store holding `keys`.
        pub fn new(keys: Vec<StoredKey>) -> Self {
            let keys = Keys {
                working: keys,
                revoked: Vec::new(),
            };
            Self {
                keys: Mutex::new(keys),
            }
        }

        fn keys(&self) -> MutexGuard<'_, Keys> {
            self.keys.lock().unwrap_or_else(PoisonError::into_inner)
        }
    }

    /// Memory answers at once, so each future is ready when returned.
    impl KeyStore for MemoryKeyStore {
        fn find<'a>(
            &'a self,
            id: &'a KeyId,
        ) -> BoxFuture<'a, Result<Option<StoredKey>, KeyStoreError>> {
            let found = self
                .keys()
                .working
                .iter()
                .find(|key| key.id() == id)
                .cloned();
            Box::pin(ready(Ok(found)))
        }

        fn create<'a>(
            &'a self,
            key: StoredKey,
            _name: &'a str,
        ) -> BoxFuture<'a, Result<(), CreateKeyError>> {
            let mut keys = self.keys();
            let taken = keys.revoked.contains(key.id())
                || keys.working.iter().any(|stored| stored.id() == key.id());
            let created = if taken {
                Err(CreateKeyError::IdTaken(key.id().clone()))
            } else {
                keys.working.push(key);
                Ok(())
            };
            Box::pin(ready(created))
        }

        fn revoke<'a>(&'a self, id: &'a KeyId) -> BoxFuture<'a, Result<bool, KeyStoreError>> {
            let mut keys = self.keys();
            let before = keys.working.len();
            keys.working.retain(|key| key.id() != id);
            let revoked = keys.working.len() < before;
            if revoked {
                keys.revoked.push(id.clone());
            }
            Box::pin(ready(Ok(revoked)))
        }
    }
}
