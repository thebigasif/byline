//! Signed-in sessions: the token in the cookie, what the store keeps of
//! it, and the CSRF token that goes with it (RULES.md BL-SEC-3, BL-SEC-9).

use std::error::Error;
use std::fmt::Write as _;
use std::str::FromStr;

use chrono::{DateTime, TimeDelta, Utc};
use khalid_api::content_store::BoxFuture;
use khalid_core::Secret;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use crate::AccountId;

/// The cookie's name. `__Host-` makes a browser keep it only when it is
/// `Secure`, has `Path=/` and names no domain, so no other site or
/// subdomain can set it.
pub const COOKIE: &str = "__Host-khalid-session";

/// The header every request signed in by cookie carries, holding the
/// session's [`CsrfToken`].
pub const CSRF_HEADER: &str = "x-khalid-csrf";

/// How long a session lasts from sign-in, however much it is used.
pub const LIFETIME: TimeDelta = TimeDelta::days(14);

/// How long a session lasts unused.
pub const IDLE: TimeDelta = TimeDelta::hours(12);

/// Random bytes in a session token: 256 bits (BL-SEC-3).
const RANDOM_BYTES: usize = 32;

/// What a session cookie holds: 64 lowercase hex digits. Only its
/// [`SessionHash`] is stored, so a copy of the database signs nobody in.
#[derive(Debug)]
pub struct SessionToken(Secret<String>);

impl SessionToken {
    /// A new token from the system's random source.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError::Random`] when there is no random source.
    pub fn generate() -> Result<Self, SessionError> {
        let mut bytes = [0_u8; RANDOM_BYTES];
        getrandom::fill(&mut bytes).map_err(|_| SessionError::Random)?;
        Ok(Self(Secret::new(hex(&bytes))))
    }

    /// What the store keeps.
    #[must_use]
    pub fn hash(&self) -> SessionHash {
        SessionHash(Sha256::digest(self.0.expose().as_bytes()).into())
    }

    /// The CSRF token of this session. It is derived from the secret, so
    /// it needs no storage, and it cannot be turned back into the token.
    #[must_use]
    pub fn csrf(&self) -> CsrfToken {
        let mut digest = Sha256::new();
        digest.update(b"khalid csrf\0");
        digest.update(self.0.expose().as_bytes());
        CsrfToken(hex(digest.finalize().as_slice()))
    }

    /// The `Set-Cookie` value that signs a browser in.
    #[must_use]
    pub fn set_cookie(&self) -> Secret<String> {
        Secret::new(format!(
            "{COOKIE}={}; Path=/; Secure; HttpOnly; SameSite=Lax; Max-Age={}",
            self.0.expose(),
            LIFETIME.num_seconds()
        ))
    }

    /// The session token in a `Cookie` header, if there is one.
    #[must_use]
    pub fn from_cookie_header(header: &str) -> Option<Self> {
        header
            .split(';')
            .filter_map(|pair| pair.trim().split_once('='))
            .find(|(name, _)| *name == COOKIE)
            .and_then(|(_, value)| value.parse().ok())
    }
}

impl FromStr for SessionToken {
    type Err = SessionError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let valid = text.len() == RANDOM_BYTES * 2
            && text
                .bytes()
                .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'));
        if valid {
            Ok(Self(Secret::new(text.to_owned())))
        } else {
            Err(SessionError::NotAToken)
        }
    }
}

/// The `Set-Cookie` value that signs a browser out.
pub const CLEAR_COOKIE: &str =
    "__Host-khalid-session=; Path=/; Secure; HttpOnly; SameSite=Lax; Max-Age=0";

/// The SHA-256 of a session token, as the store keeps it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SessionHash([u8; 32]);

impl SessionHash {
    /// A hash read back from the store.
    #[must_use]
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// The bytes, for a store to bind.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// A session's CSRF token, sent back in [`CSRF_HEADER`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsrfToken(String);

impl CsrfToken {
    /// The token, for the sign-in answer.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether `presented` is this token, in the same time whichever byte
    /// differs.
    #[must_use]
    pub fn matches(&self, presented: &str) -> bool {
        bool::from(self.0.as_bytes().ct_eq(presented.as_bytes()))
    }
}

/// A session as the store keeps it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoredSession {
    /// Whose session it is.
    pub account: AccountId,
    /// When it began.
    pub created_at: DateTime<Utc>,
    /// When it was last used.
    pub last_seen_at: DateTime<Utc>,
}

impl StoredSession {
    /// Whether it still signs someone in at `now`: younger than
    /// [`LIFETIME`] and used within [`IDLE`].
    #[must_use]
    pub fn is_live(&self, now: DateTime<Utc>) -> bool {
        now < self.created_at + LIFETIME && now < self.last_seen_at + IDLE
    }
}

/// The sessions of one site. `khalid-db` keeps them in `sessions`.
pub trait SessionStore: Send + Sync {
    /// Stores a new session for `account`, begun at `at`.
    ///
    /// # Errors
    ///
    /// Returns [`SessionStoreError`] when the store cannot be written.
    fn begin(
        &self,
        hash: SessionHash,
        account: AccountId,
        at: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<(), SessionStoreError>>;

    /// The session with this hash, live or not.
    ///
    /// # Errors
    ///
    /// Returns [`SessionStoreError`] when the store cannot be read.
    fn find(
        &self,
        hash: SessionHash,
    ) -> BoxFuture<'_, Result<Option<StoredSession>, SessionStoreError>>;

    /// Records that the session was used at `at`.
    ///
    /// # Errors
    ///
    /// Returns [`SessionStoreError`] when the store cannot be written.
    fn touch(
        &self,
        hash: SessionHash,
        at: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<(), SessionStoreError>>;

    /// Ends one session. Returns whether it existed.
    ///
    /// # Errors
    ///
    /// Returns [`SessionStoreError`] when the store cannot be written.
    fn end(&self, hash: SessionHash) -> BoxFuture<'_, Result<bool, SessionStoreError>>;

    /// Ends every session of `account` except `keep`. Returns how many
    /// ended.
    ///
    /// # Errors
    ///
    /// Returns [`SessionStoreError`] when the store cannot be written.
    fn end_all(
        &self,
        account: AccountId,
        keep: Option<SessionHash>,
    ) -> BoxFuture<'_, Result<u64, SessionStoreError>>;
}

/// The sessions could not be read or written.
#[derive(Debug, thiserror::Error)]
#[error("the sessions could not be read or written")]
pub struct SessionStoreError {
    #[source]
    source: Box<dyn Error + Send + Sync>,
}

impl SessionStoreError {
    /// Wraps what the storage reported.
    pub fn new(source: impl Error + Send + Sync + 'static) -> Self {
        Self {
            source: Box::new(source),
        }
    }
}

/// A session token could not be made or read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SessionError {
    /// The system has no random source.
    #[error("the system's random source is not available")]
    Random,
    /// The text is not 64 lowercase hex digits.
    #[error("this is not a session token")]
    NotAToken,
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut text, byte| {
            let _ = write!(text, "{byte:02x}");
            text
        })
}
