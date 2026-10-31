//! The keys the API checks for `khalid mcp`: the site's own, and one
//! content key the server makes for itself.

use std::future::ready;

use khalid_api::content_store::BoxFuture;
use khalid_api::keys::{
    CreateKeyError, GenerateKeyError, KeyId, KeyKind, KeyStore, KeyStoreError, KeyToken, ListedKey,
    StoredKey,
};
use khalid_core::Secret;

/// The site's keys, plus a content key that exists only in this process,
/// so the server's own list requests pass the Content API's key check. It
/// is never stored, and only [`SessionKeys::new`] ever shows it.
#[derive(Debug)]
pub struct SessionKeys<K> {
    reader: StoredKey,
    keys: K,
}

impl<K: KeyStore> SessionKeys<K> {
    /// The site's `keys` and a new content key, which is returned once.
    ///
    /// # Errors
    ///
    /// Returns [`GenerateKeyError`] when the system has no random bytes
    /// to make the key from.
    pub fn new(keys: K) -> Result<(Self, Secret<String>), GenerateKeyError> {
        let token = KeyToken::generate(KeyKind::Content)?;
        let reader = StoredKey::for_token(&token);
        Ok((Self { reader, keys }, token.reveal()))
    }
}

impl<K: KeyStore> KeyStore for SessionKeys<K> {
    fn find<'a>(
        &'a self,
        id: &'a KeyId,
    ) -> BoxFuture<'a, Result<Option<StoredKey>, KeyStoreError>> {
        if id == self.reader.id() {
            return Box::pin(ready(Ok(Some(self.reader.clone()))));
        }
        self.keys.find(id)
    }

    fn create<'a>(
        &'a self,
        key: StoredKey,
        name: &'a str,
    ) -> BoxFuture<'a, Result<(), CreateKeyError>> {
        self.keys.create(key, name)
    }

    fn revoke<'a>(&'a self, id: &'a KeyId) -> BoxFuture<'a, Result<bool, KeyStoreError>> {
        self.keys.revoke(id)
    }

    /// The site's keys; the session's own content key is never stored, so
    /// it is never listed.
    fn list(&self) -> BoxFuture<'_, Result<Vec<ListedKey>, KeyStoreError>> {
        self.keys.list()
    }
}
