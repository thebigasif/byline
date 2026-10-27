//! A staff account, and where accounts are kept.

use std::error::Error;

use chrono::{DateTime, Utc};
use khalid_api::content_store::BoxFuture;
use khalid_api::keys::Role;

use crate::{Email, PasswordHash};

/// An account's row in the store. It is never shown to a reader.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AccountId(i64);

impl AccountId {
    /// The account with row `row`, as the store numbers them.
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

/// A staff member: who they are and what role they hold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    id: AccountId,
    email: Email,
    name: String,
    role: Role,
}

impl Account {
    /// An account read back from the store.
    #[must_use]
    pub fn new(id: AccountId, email: Email, name: String, role: Role) -> Self {
        Self {
            id,
            email,
            name,
            role,
        }
    }

    /// The row.
    #[must_use]
    pub fn id(&self) -> AccountId {
        self.id
    }

    /// The address they sign in with.
    #[must_use]
    pub fn email(&self) -> &Email {
        &self.email
    }

    /// The name the admin shows.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Their role.
    #[must_use]
    pub fn role(&self) -> Role {
        self.role
    }

    #[cfg(any(test, feature = "test-util"))]
    pub(crate) fn set_role(&mut self, role: Role) {
        self.role = role;
    }
}

/// An account with its password hash, for signing in. `None` means the
/// account signs in with a passkey or a magic link only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredAccount {
    /// The account.
    pub account: Account,
    /// Its password hash, if it has a password.
    pub password: Option<PasswordHash>,
}

/// The staff accounts of one site. `khalid-db` keeps them on SQLite as
/// `AccountTable`; `MemoryAccounts`, from the `test-util` feature, in
/// memory.
pub trait AccountStore: Send + Sync {
    /// Adds an account with no password, made at `at`.
    ///
    /// # Errors
    ///
    /// Returns [`CreateAccountError::EmailTaken`] when an account has this
    /// email, [`CreateAccountError::OwnerTaken`] for a second owner, and
    /// [`CreateAccountError::Unavailable`] when the store fails.
    fn create<'a>(
        &'a self,
        email: &'a Email,
        name: &'a str,
        role: Role,
        at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<Account, CreateAccountError>>;

    /// The account with this email, with its password hash.
    ///
    /// # Errors
    ///
    /// Returns [`AccountStoreError`] when the store cannot be read.
    fn find_by_email<'a>(
        &'a self,
        email: &'a Email,
    ) -> BoxFuture<'a, Result<Option<StoredAccount>, AccountStoreError>>;

    /// The account with row `id`.
    ///
    /// # Errors
    ///
    /// Returns [`AccountStoreError`] when the store cannot be read.
    fn find(&self, id: AccountId) -> BoxFuture<'_, Result<Option<Account>, AccountStoreError>>;

    /// Every account, by email.
    ///
    /// # Errors
    ///
    /// Returns [`AccountStoreError`] when the store cannot be read.
    fn list(&self) -> BoxFuture<'_, Result<Vec<Account>, AccountStoreError>>;

    /// Gives account `id` role `role`. Returns whether the account exists.
    /// Never makes or unmakes the owner; that is [`Self::transfer`].
    ///
    /// # Errors
    ///
    /// Returns [`AccountStoreError`] when the store cannot be written.
    fn set_role(&self, id: AccountId, role: Role)
    -> BoxFuture<'_, Result<bool, AccountStoreError>>;

    /// Stores `hash` as account `id`'s password. Returns whether the
    /// account exists.
    ///
    /// # Errors
    ///
    /// Returns [`AccountStoreError`] when the store cannot be written.
    fn set_password<'a>(
        &'a self,
        id: AccountId,
        hash: &'a PasswordHash,
    ) -> BoxFuture<'a, Result<bool, AccountStoreError>>;

    /// Removes account `id` with its sessions, passkeys and links. Returns
    /// whether it existed.
    ///
    /// # Errors
    ///
    /// Returns [`AccountStoreError`] when the store cannot be written.
    fn remove(&self, id: AccountId) -> BoxFuture<'_, Result<bool, AccountStoreError>>;

    /// Makes `to` the owner and `from`, the owner until now, an admin, in
    /// one write. Returns `false`, changing nothing, unless `from` is the
    /// owner and `to` another account.
    ///
    /// # Errors
    ///
    /// Returns [`AccountStoreError`] when the store cannot be written.
    fn transfer(
        &self,
        from: AccountId,
        to: AccountId,
    ) -> BoxFuture<'_, Result<bool, AccountStoreError>>;
}

/// A new account could not be stored.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CreateAccountError {
    /// Another account has this email.
    #[error("an account already has this email")]
    EmailTaken,
    /// The new account would be a second owner. A site has one.
    #[error("the site already has an owner")]
    OwnerTaken,
    /// The store could not be written.
    #[error("the account could not be stored")]
    Unavailable {
        /// What the store reported.
        #[source]
        source: AccountStoreError,
    },
}

/// The accounts could not be read or written.
#[derive(Debug, thiserror::Error)]
#[error("the staff accounts could not be read or written")]
pub struct AccountStoreError {
    #[source]
    source: Box<dyn Error + Send + Sync>,
}

impl AccountStoreError {
    /// Wraps what the storage reported.
    pub fn new(source: impl Error + Send + Sync + 'static) -> Self {
        Self {
            source: Box::new(source),
        }
    }
}
