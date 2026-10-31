//! Where staff passkeys are kept.

use std::error::Error;

use chrono::{DateTime, Utc};
use khalid_api::content_store::BoxFuture;

use crate::{AccountId, SavedPasskey};

/// A passkey as the store keeps it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredPasskey {
    /// Whose passkey it is.
    pub account: AccountId,
    /// The passkey, as `webauthn-rs` saves it.
    pub passkey: SavedPasskey,
    /// The name the person gave it, such as "Work laptop".
    pub name: String,
}

/// One passkey's number in its store, which the admin names it by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PasskeyId(i64);

impl PasskeyId {
    /// The passkey with row `row`, as the store numbers them.
    #[must_use]
    pub fn from_row(row: i64) -> Self {
        Self(row)
    }

    /// The row, for a store to bind.
    #[must_use]
    pub fn as_row(self) -> i64 {
        self.0
    }
}

/// The most characters a passkey's name may have.
pub const MAX_NAME_CHARS: usize = 60;

/// A passkey's name, such as "Work laptop": trimmed, one to
/// [`MAX_NAME_CHARS`] characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PasskeyName(String);

impl PasskeyName {
    /// `text` without the spaces around it, as a passkey's name.
    ///
    /// # Errors
    ///
    /// Returns [`PasskeyNameError`] when nothing is left, or more than
    /// [`MAX_NAME_CHARS`] characters are.
    pub fn new(text: &str) -> Result<Self, PasskeyNameError> {
        let name = text.trim();
        let chars = name.chars().count();
        if chars == 0 || chars > MAX_NAME_CHARS || name.chars().any(char::is_control) {
            return Err(PasskeyNameError);
        }
        Ok(Self(name.to_owned()))
    }

    /// The name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A passkey's name was blank, too long, or held a control character.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("a passkey's name has 1 to {MAX_NAME_CHARS} characters")]
pub struct PasskeyNameError;

/// A passkey as its owner sees it in the admin: never its key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListedPasskey {
    /// Its number in the store.
    pub id: PasskeyId,
    /// The name the person gave it.
    pub name: String,
    /// When it was added.
    pub added_at: DateTime<Utc>,
    /// When it last signed someone in, if ever.
    pub last_used_at: Option<DateTime<Utc>>,
}

/// The passkeys of one site. `khalid-db` keeps them in `passkeys`.
pub trait PasskeyStore: Send + Sync {
    /// Stores `passkey` as a passkey of `account` named `name`, made at
    /// `at`. Returns `false`, storing nothing, when a passkey with this
    /// credential is already stored.
    ///
    /// # Errors
    ///
    /// Returns [`PasskeyStoreError`] when the store cannot be written.
    fn add<'a>(
        &'a self,
        account: AccountId,
        passkey: &'a SavedPasskey,
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

    /// Records a sign-in at `at` with the passkey of the same credential,
    /// saving `passkey` as it now stands, with its new counter.
    ///
    /// # Errors
    ///
    /// Returns [`PasskeyStoreError`] when the store cannot be written.
    fn used<'a>(
        &'a self,
        passkey: &'a SavedPasskey,
        at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<(), PasskeyStoreError>>;

    /// Every passkey of `account`, oldest first, as its owner sees them.
    ///
    /// # Errors
    ///
    /// Returns [`PasskeyStoreError`] when the store cannot be read.
    fn listed(
        &self,
        account: AccountId,
    ) -> BoxFuture<'_, Result<Vec<ListedPasskey>, PasskeyStoreError>>;

    /// Names the passkey `id` of `account` `name`. Returns `false`,
    /// changing nothing, when `account` has no passkey `id`.
    ///
    /// # Errors
    ///
    /// Returns [`PasskeyStoreError`] when the store cannot be written.
    fn rename<'a>(
        &'a self,
        account: AccountId,
        id: PasskeyId,
        name: &'a str,
    ) -> BoxFuture<'a, Result<bool, PasskeyStoreError>>;

    /// Removes the passkey `id` of `account`, so it signs nobody in.
    /// Returns `false` when `account` has no passkey `id`.
    ///
    /// # Errors
    ///
    /// Returns [`PasskeyStoreError`] when the store cannot be written.
    fn remove(
        &self,
        account: AccountId,
        id: PasskeyId,
    ) -> BoxFuture<'_, Result<bool, PasskeyStoreError>>;
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
