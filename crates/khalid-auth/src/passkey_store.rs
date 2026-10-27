//! Where staff passkeys are kept.

use std::error::Error;

use chrono::{DateTime, Utc};
use khalid_api::content_store::BoxFuture;

use crate::{AccountId, PublicKey, Registration};

/// A passkey as the store keeps it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredPasskey {
    /// Whose passkey it is.
    pub account: AccountId,
    /// The authenticator's name for it.
    pub credential_id: Vec<u8>,
    /// Its public key.
    pub public_key: PublicKey,
    /// Its signature counter so far.
    pub sign_count: u32,
    /// The name the person gave it, such as "Work laptop".
    pub name: String,
}

/// The passkeys of one site. `khalid-db` keeps them in `passkeys`.
pub trait PasskeyStore: Send + Sync {
    /// Stores `registration` as a passkey of `account` named `name`, made
    /// at `at`. Returns `false`, storing nothing, when a passkey with this
    /// credential is already stored.
    ///
    /// # Errors
    ///
    /// Returns [`PasskeyStoreError`] when the store cannot be written.
    fn add<'a>(
        &'a self,
        account: AccountId,
        registration: &'a Registration,
        name: &'a str,
        at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<bool, PasskeyStoreError>>;

    /// The passkey with this credential.
    ///
    /// # Errors
    ///
    /// Returns [`PasskeyStoreError`] when the store cannot be read.
    fn find<'a>(
        &'a self,
        credential_id: &'a [u8],
    ) -> BoxFuture<'a, Result<Option<StoredPasskey>, PasskeyStoreError>>;

    /// Every passkey of `account`, oldest first.
    ///
    /// # Errors
    ///
    /// Returns [`PasskeyStoreError`] when the store cannot be read.
    fn of(
        &self,
        account: AccountId,
    ) -> BoxFuture<'_, Result<Vec<StoredPasskey>, PasskeyStoreError>>;

    /// Records a sign-in with this credential at `at`, with its new
    /// counter.
    ///
    /// # Errors
    ///
    /// Returns [`PasskeyStoreError`] when the store cannot be written.
    fn used<'a>(
        &'a self,
        credential_id: &'a [u8],
        sign_count: u32,
        at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<(), PasskeyStoreError>>;
}

/// The passkeys could not be read or written.
#[derive(Debug, thiserror::Error)]
#[error("the passkeys could not be read or written")]
pub struct PasskeyStoreError {
    #[source]
    source: Box<dyn Error + Send + Sync>,
}

impl PasskeyStoreError {
    /// Wraps what the storage reported.
    pub fn new(source: impl Error + Send + Sync + 'static) -> Self {
        Self {
            source: Box::new(source),
        }
    }
}
