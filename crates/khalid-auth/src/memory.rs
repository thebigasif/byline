//! Stores kept in memory, for tests in this crate and others.

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Mutex, MutexGuard};

use chrono::{DateTime, Utc};
use khalid_api::content_store::BoxFuture;
use khalid_api::keys::Role;
use khalid_core::Secret;

use crate::{
    Account, AccountId, AccountStore, AccountStoreError, CreateAccountError, Email, LINK_LIFETIME,
    LinkSender, LinkStore, LinkStoreError, ListedPasskey, PasskeyId, PasskeyStore,
    PasskeyStoreError, PasswordHash, SavedPasskey, SendLinkError, SessionHash, SessionStore,
    SessionStoreError, StoredAccount, StoredPasskey, StoredSession,
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

/// Sessions in memory.
#[derive(Debug, Default)]
pub struct MemorySessions {
    rows: Mutex<Vec<(SessionHash, StoredSession)>>,
}

impl MemorySessions {
    /// No sessions yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// How many sessions are stored, live or not.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rows().len()
    }

    /// Whether no session is stored.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[expect(
        clippy::expect_used,
        reason = "a test double; a poisoned lock means a test already failed"
    )]
    fn rows(&self) -> MutexGuard<'_, Vec<(SessionHash, StoredSession)>> {
        self.rows.lock().expect("lock poisoned")
    }
}

impl SessionStore for MemorySessions {
    fn begin(
        &self,
        hash: SessionHash,
        account: AccountId,
        at: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<(), SessionStoreError>> {
        let session = StoredSession {
            account,
            created_at: at,
            last_seen_at: at,
        };
        self.rows().push((hash, session));
        ready(Ok(()))
    }

    fn find(
        &self,
        hash: SessionHash,
    ) -> BoxFuture<'_, Result<Option<StoredSession>, SessionStoreError>> {
        let rows = self.rows();
        let found = rows
            .iter()
            .find(|(row, _)| *row == hash)
            .map(|(_, session)| *session);
        ready(Ok(found))
    }

    fn touch(
        &self,
        hash: SessionHash,
        at: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<(), SessionStoreError>> {
        for (row, session) in self.rows().iter_mut() {
            if *row == hash {
                session.last_seen_at = at;
            }
        }
        ready(Ok(()))
    }

    fn end(&self, hash: SessionHash) -> BoxFuture<'_, Result<bool, SessionStoreError>> {
        let mut rows = self.rows();
        let before = rows.len();
        rows.retain(|(row, _)| *row != hash);
        ready(Ok(rows.len() < before))
    }

    fn end_all(
        &self,
        account: AccountId,
        keep: Option<SessionHash>,
    ) -> BoxFuture<'_, Result<u64, SessionStoreError>> {
        let mut rows = self.rows();
        let before = rows.len();
        rows.retain(|(row, session)| session.account != account || Some(*row) == keep);
        let ended = u64::try_from(before - rows.len()).unwrap_or(u64::MAX);
        ready(Ok(ended))
    }
}

/// Sign-in links in memory.
#[derive(Debug, Default)]
pub struct MemoryLinks {
    /// Each unused link: its hash, its account, and when it was made.
    rows: Mutex<Vec<(SessionHash, AccountId, DateTime<Utc>)>>,
}

impl MemoryLinks {
    /// No links yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[expect(
        clippy::expect_used,
        reason = "a test double; a poisoned lock means a test already failed"
    )]
    fn rows(&self) -> MutexGuard<'_, Vec<(SessionHash, AccountId, DateTime<Utc>)>> {
        self.rows.lock().expect("lock poisoned")
    }
}

impl LinkStore for MemoryLinks {
    fn create(
        &self,
        hash: SessionHash,
        account: AccountId,
        at: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<(), LinkStoreError>> {
        self.rows().push((hash, account, at));
        ready(Ok(()))
    }

    fn redeem(
        &self,
        hash: SessionHash,
        at: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<Option<AccountId>, LinkStoreError>> {
        let mut rows = self.rows();
        let found = rows.iter().position(|(row, _, _)| *row == hash);
        // A used link is gone; an expired one is found and refused.
        let redeemed = found.map(|index| rows.swap_remove(index));
        let account = redeemed
            .filter(|(_, _, made)| at < *made + LINK_LIFETIME)
            .map(|(_, account, _)| account);
        ready(Ok(account))
    }
}

/// A mailbox that keeps every sign-in link it is asked to send.
#[derive(Debug, Default)]
pub struct MemoryMailbox {
    sent: Mutex<Vec<(Email, String)>>,
}

impl MemoryMailbox {
    /// Nothing sent yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Every link sent, with its address, in order.
    ///
    /// # Panics
    ///
    /// When a test panicked while sending, which poisons the mailbox.
    #[must_use]
    #[expect(
        clippy::expect_used,
        reason = "a test double; a poisoned lock means a test already failed"
    )]
    pub fn sent(&self) -> Vec<(Email, String)> {
        self.sent.lock().expect("lock poisoned").clone()
    }
}

impl LinkSender for MemoryMailbox {
    #[expect(
        clippy::expect_used,
        reason = "a test double; a poisoned lock means a test already failed"
    )]
    fn send<'a>(
        &'a self,
        to: &'a Email,
        link: Secret<String>,
    ) -> BoxFuture<'a, Result<(), SendLinkError>> {
        let mut sent = self.sent.lock().expect("lock poisoned");
        sent.push((to.clone(), link.expose().clone()));
        ready(Ok(()))
    }
}

/// One passkey in memory, with what the admin lists.
#[derive(Debug)]
struct PasskeyRow {
    stored: StoredPasskey,
    listed: ListedPasskey,
}

/// Passkeys in memory, oldest first, numbered from 1 in the order they
/// are added.
#[derive(Debug, Default)]
pub struct MemoryPasskeys {
    rows: Mutex<Vec<PasskeyRow>>,
    added: AtomicI64,
}

impl MemoryPasskeys {
    /// No passkeys yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[expect(
        clippy::expect_used,
        reason = "a test double; a poisoned lock means a test already failed"
    )]
    fn rows(&self) -> MutexGuard<'_, Vec<PasskeyRow>> {
        self.rows.lock().expect("lock poisoned")
    }
}

impl PasskeyRow {
    fn credential_id(&self) -> &[u8] {
        self.stored.passkey.credential_id()
    }

    fn is(&self, account: AccountId, id: PasskeyId) -> bool {
        self.stored.account == account && self.listed.id == id
    }
}

impl PasskeyStore for MemoryPasskeys {
    fn add<'a>(
        &'a self,
        account: AccountId,
        passkey: &'a SavedPasskey,
        name: &'a str,
        at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<bool, PasskeyStoreError>> {
        let mut rows = self.rows();
        let id = passkey.credential_id();
        if rows.iter().any(|row| row.credential_id() == id) {
            return ready(Ok(false));
        }
        let number = self.added.fetch_add(1, Ordering::Relaxed).saturating_add(1);
        rows.push(PasskeyRow {
            stored: StoredPasskey {
                account,
                passkey: passkey.clone(),
                name: name.to_owned(),
            },
            listed: ListedPasskey {
                id: PasskeyId::from_row(number),
                name: name.to_owned(),
                added_at: at,
                last_used_at: None,
            },
        });
        ready(Ok(true))
    }

    fn find<'a>(
        &'a self,
        credential_id: &'a [u8],
    ) -> BoxFuture<'a, Result<Option<StoredPasskey>, PasskeyStoreError>> {
        let rows = self.rows();
        let found = rows.iter().find(|row| row.credential_id() == credential_id);
        ready(Ok(found.map(|row| row.stored.clone())))
    }

    fn of(
        &self,
        account: AccountId,
    ) -> BoxFuture<'_, Result<Vec<StoredPasskey>, PasskeyStoreError>> {
        let rows = self.rows();
        let mine = rows.iter().filter(|row| row.stored.account == account);
        ready(Ok(mine.map(|row| row.stored.clone()).collect()))
    }

    fn used<'a>(
        &'a self,
        passkey: &'a SavedPasskey,
        at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<(), PasskeyStoreError>> {
        let mut rows = self.rows();
        let id = passkey.credential_id();
        for row in rows.iter_mut().filter(|row| row.credential_id() == id) {
            row.stored.passkey = passkey.clone();
            row.listed.last_used_at = Some(at);
        }
        ready(Ok(()))
    }

    fn listed(
        &self,
        account: AccountId,
    ) -> BoxFuture<'_, Result<Vec<ListedPasskey>, PasskeyStoreError>> {
        let rows = self.rows();
        let mine = rows.iter().filter(|row| row.stored.account == account);
        ready(Ok(mine.map(|row| row.listed.clone()).collect()))
    }

    fn rename<'a>(
        &'a self,
        account: AccountId,
        id: PasskeyId,
        name: &'a str,
    ) -> BoxFuture<'a, Result<bool, PasskeyStoreError>> {
        let mut rows = self.rows();
        let Some(row) = rows.iter_mut().find(|row| row.is(account, id)) else {
            return ready(Ok(false));
        };
        name.clone_into(&mut row.stored.name);
        name.clone_into(&mut row.listed.name);
        ready(Ok(true))
    }

    fn remove(
        &self,
        account: AccountId,
        id: PasskeyId,
    ) -> BoxFuture<'_, Result<bool, PasskeyStoreError>> {
        let mut rows = self.rows();
        let before = rows.len();
        rows.retain(|row| !row.is(account, id));
        ready(Ok(rows.len() < before))
    }
}
