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
mod magic_link;
#[cfg(any(test, feature = "test-util"))]
mod memory;
mod passkey;
mod passkey_site;
mod passkey_store;
mod password;
mod permission;
mod routes;
mod session;
mod session_door;

pub use account::{
    Account, AccountId, AccountStore, AccountStoreError, CreateAccountError, StoredAccount,
};
pub use accounts::{
    Accounts, CEREMONY, FRESH, Links, Passkeys, RegistrationRequest, SignInError, SignedIn,
};
pub use email::{Email, ParseEmailError};
pub use limiter::{MAX_FAILURES, SignInLimiter, WINDOW};
pub use magic_link::{
    LINK_LIFETIME, LinkSender, LinkStore, LinkStoreError, LinkToken, SendLinkError,
};
#[cfg(any(test, feature = "test-util"))]
pub use memory::{MemoryAccounts, MemoryLinks, MemoryMailbox, MemoryPasskeys, MemorySessions};
pub use passkey::{
    AuthenticatorData, Challenge, PasskeyError, PublicKey, Registration, RelyingParty,
    verify_assertion, verify_registration,
};
pub use passkey_site::{PasskeyCheckError, PasskeySite, Registering, SavedPasskey, SigningIn};
pub use passkey_store::{PasskeyStore, PasskeyStoreError, StoredPasskey};
pub use password::{
    MAX_BYTES, MIN_CHARS, Password, PasswordError, PasswordHash, PasswordHashing, WeakPasswordError,
};
pub use permission::{ParsePermissionError, Permission, may_assign, may_manage};
pub use routes::staff_routes;
pub use session::{
    CLEAR_COOKIE, COOKIE, CSRF_HEADER, CsrfToken, IDLE, LIFETIME, SessionError, SessionHash,
    SessionStore, SessionStoreError, SessionToken, StoredSession,
};
/// The `webauthn-rs` types the passkey ceremonies send and receive, so
/// callers name the same version.
pub use webauthn_rs::prelude::{
    CreationChallengeResponse, PublicKeyCredential, RegisterPublicKeyCredential,
    RequestChallengeResponse, WebauthnError,
};
