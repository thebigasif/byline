//! API keys: what a key looks like, how it is stored, and how a request's
//! key is checked (RULES.md BL-SEC-3, ADR 0011).

mod key_store;
mod key_token;
mod stored_key;

#[cfg(any(test, feature = "test-util"))]
pub use key_store::MemoryKeyStore;
pub use key_store::{KeyError, KeyStore, KeyStoreError, authenticate};
pub use key_token::{KeyId, KeyKind, KeyToken, ParseKeyError};
pub use stored_key::StoredKey;
