//! Staff accounts for the admin (plan section 4.6, PRD KH-EDT-7).
//!
//! This crate owns who may sign in and what each person may do: the
//! [`Permission`] table for the five roles, which roles each role may
//! manage, the [`Account`] with its [`Email`] and [`Password`], and the
//! [`AccountStore`] that keeps accounts. The roles themselves are
//! [`khalid_api::keys::Role`], so a staff token and a signed-in account
//! with the same role may do the same. It must not know about content
//! files, templates or SQL; `khalid-db` stores the accounts behind the
//! traits this crate defines.

mod account;
mod accounts;
mod email;
mod limiter;
#[cfg(any(test, feature = "test-util"))]
mod memory;
mod password;
mod permission;
mod session;

pub use account::{
    Account, AccountId, AccountStore, AccountStoreError, CreateAccountError, StoredAccount,
};
pub use accounts::{Accounts, FRESH, SignInError, SignedIn};
pub use email::{Email, ParseEmailError};
pub use limiter::{MAX_FAILURES, SignInLimiter, WINDOW};
#[cfg(any(test, feature = "test-util"))]
pub use memory::{MemoryAccounts, MemorySessions};
pub use password::{
    MAX_BYTES, MIN_CHARS, Password, PasswordError, PasswordHash, PasswordHashing, WeakPasswordError,
};
pub use permission::{ParsePermissionError, Permission, may_assign, may_manage};
pub use session::{
    CLEAR_COOKIE, COOKIE, CSRF_HEADER, CsrfToken, IDLE, LIFETIME, SessionError, SessionHash,
    SessionStore, SessionStoreError, SessionToken, StoredSession,
};
