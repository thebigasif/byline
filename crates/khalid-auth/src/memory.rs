//! Stores kept in memory, for tests in this crate and others.

use std::sync::{Mutex, MutexGuard};

use chrono::{DateTime, Utc};
use khalid_api::content_store::BoxFuture;
use khalid_api::keys::Role;

use crate::{
    Account, AccountId, AccountStore, AccountStoreError, CreateAccountError, Email, PasswordHash,
    StoredAccount,
};

/// Staff accounts in memory, numbered from 1 in the order they are made.
#[derive(Debug, Default)]
pub struct MemoryAccounts {
    rows: Mutex<Vec<StoredAccount>>,
}

impl MemoryAccounts {
    /// No accounts yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[expect(
        clippy::expect_used,
        reason = "a test double; a poisoned lock means a test already failed"
    )]
    fn rows(&self) -> MutexGuard<'_, Vec<StoredAccount>> {
        self.rows.lock().expect("lock poisoned")
    }

    /// Runs `change` on account `id`, and says whether it exists.
    fn change(&self, id: AccountId, change: impl FnOnce(&mut StoredAccount)) -> bool {
        let mut rows = self.rows();
        rows.iter_mut()
            .find(|row| row.account.id() == id)
            .map(change)
            .is_some()
    }

    fn role_of(&self, id: AccountId) -> Option<Role> {
        let rows = self.rows();
        let row = rows.iter().find(|row| row.account.id() == id)?;
        Some(row.account.role())
    }
}

/// Answers `value` from a future, the way a real store would.
fn ready<'a, T: Send + 'a>(value: T) -> BoxFuture<'a, T> {
    Box::pin(async move { value })
}

impl AccountStore for MemoryAccounts {
    fn create<'a>(
        &'a self,
        email: &'a Email,
        name: &'a str,
        role: Role,
        _at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<Account, CreateAccountError>> {
        let mut rows = self.rows();
        if rows.iter().any(|row| row.account.email() == email) {
            return ready(Err(CreateAccountError::EmailTaken));
        }
        if role == Role::Owner && rows.iter().any(|row| row.account.role() == Role::Owner) {
            return ready(Err(CreateAccountError::OwnerTaken));
        }
        let last = rows.iter().map(|row| row.account.id().as_row()).max();
        let id = AccountId::from_row(last.unwrap_or(0) + 1);
        let account = Account::new(id, email.clone(), name.to_owned(), role);
        rows.push(StoredAccount {
            account: account.clone(),
            password: None,
        });
        ready(Ok(account))
    }

    fn find_by_email<'a>(
        &'a self,
        email: &'a Email,
    ) -> BoxFuture<'a, Result<Option<StoredAccount>, AccountStoreError>> {
        let rows = self.rows();
        ready(Ok(rows
            .iter()
            .find(|row| row.account.email() == email)
            .cloned()))
    }

    fn find(&self, id: AccountId) -> BoxFuture<'_, Result<Option<Account>, AccountStoreError>> {
        let rows = self.rows();
        let found = rows.iter().find(|row| row.account.id() == id);
        ready(Ok(found.map(|row| row.account.clone())))
    }

    fn list(&self) -> BoxFuture<'_, Result<Vec<Account>, AccountStoreError>> {
        let mut accounts: Vec<Account> =
            self.rows().iter().map(|row| row.account.clone()).collect();
        accounts.sort_by(|a, b| a.email().cmp(b.email()));
        ready(Ok(accounts))
    }

    fn set_role(
        &self,
        id: AccountId,
        role: Role,
    ) -> BoxFuture<'_, Result<bool, AccountStoreError>> {
        let allowed = role != Role::Owner && self.role_of(id).is_some_and(|old| old != Role::Owner);
        ready(Ok(
            allowed && self.change(id, |row| row.account.set_role(role))
        ))
    }

    fn set_password<'a>(
        &'a self,
        id: AccountId,
        hash: &'a PasswordHash,
    ) -> BoxFuture<'a, Result<bool, AccountStoreError>> {
        ready(Ok(self.change(id, |row| row.password = Some(hash.clone()))))
    }

    fn remove(&self, id: AccountId) -> BoxFuture<'_, Result<bool, AccountStoreError>> {
        let mut rows = self.rows();
        let before = rows.len();
        rows.retain(|row| row.account.id() != id);
        ready(Ok(rows.len() < before))
    }

    fn transfer(
        &self,
        from: AccountId,
        to: AccountId,
    ) -> BoxFuture<'_, Result<bool, AccountStoreError>> {
        let valid =
            from != to && self.role_of(from) == Some(Role::Owner) && self.role_of(to).is_some();
        if valid {
            self.change(from, |row| row.account.set_role(Role::Admin));
            self.change(to, |row| row.account.set_role(Role::Owner));
        }
        ready(Ok(valid))
    }
}
