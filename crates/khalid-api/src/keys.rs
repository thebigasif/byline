//! API keys: what a key looks like, how it is stored, what it may do, and
//! how a request's key is checked (RULES.md BL-SEC-3, ADR 0011).

mod access;
mod key_store;
mod key_token;
mod stored_key;

pub use access::{Access, ParseRoleError, ParseScopeError, Role, Scope};
#[cfg(any(test, feature = "test-util"))]
pub use key_store::MemoryKeyStore;
pub use key_store::{CreateKeyError, KeyError, KeyStore, KeyStoreError, Principal, authenticate};
pub use key_token::{GenerateKeyError, KeyId, KeyKind, KeyToken, ParseKeyError, RANDOM_BYTES};
pub use stored_key::{HashKeyError, KeyHash, KeyHashing, StoredKey};
