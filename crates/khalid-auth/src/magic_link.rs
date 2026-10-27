//! Signing in by a link sent by email: the token in the link, what the
//! store keeps of it, and the sender that mails it.

use std::error::Error;

use chrono::{DateTime, TimeDelta, Utc};
use khalid_api::content_store::BoxFuture;
use khalid_core::Secret;

use crate::{AccountId, Email, SessionError, SessionHash, SessionToken};

/// How long a sign-in link works after it is sent.
pub const LINK_LIFETIME: TimeDelta = TimeDelta::minutes(15);

/// The token in a sign-in link: 256 random bits, like a session token, and
/// stored the same way, as its SHA-256 only.
#[derive(Debug)]
pub struct LinkToken(SessionToken);

impl LinkToken {
    /// A new token from the system's random source.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError::Random`] when there is no random source.
    pub fn generate() -> Result<Self, SessionError> {
        SessionToken::generate().map(Self)
    }

    /// The token as it was sent, read back from the link.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError::NotAToken`] for anything else.
    pub fn parse(text: &str) -> Result<Self, SessionError> {
        text.parse().map(Self)
    }

    /// What the store keeps.
    #[must_use]
    pub fn hash(&self) -> SessionHash {
        self.0.hash()
    }

    /// The link to mail: `admin` followed by the page that signs in, with
    /// the token after `#`, so it never reaches a server log and a mail
    /// scanner that opens the link cannot use it up.
    #[must_use]
    pub fn link(&self, admin: &str) -> Secret<String> {
        let admin = admin.trim_end_matches('/');
        Secret::new(format!("{admin}/sign-in/link#token={}", self.0.expose()))
    }
}

/// Mails a sign-in link. The newsletter track's mail sender implements it
/// once it lands; until then a site has no link sign-in.
pub trait LinkSender: Send + Sync {
    /// Sends `link` to `to`.
    ///
    /// # Errors
    ///
    /// Returns [`SendLinkError`] when the mail cannot be handed over.
    fn send<'a>(
        &'a self,
        to: &'a Email,
        link: Secret<String>,
    ) -> BoxFuture<'a, Result<(), SendLinkError>>;
}

/// The sign-in links of one site. `khalid-db` keeps them in `magic_links`.
pub trait LinkStore: Send + Sync {
    /// Stores a link for `account`, made at `at`, working for
    /// [`LINK_LIFETIME`].
    ///
    /// # Errors
    ///
    /// Returns [`LinkStoreError`] when the store cannot be written.
    fn create(
        &self,
        hash: SessionHash,
        account: AccountId,
        at: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<(), LinkStoreError>>;

    /// Uses the link with this hash at `at`, in one write: returns its
    /// account if it was unused and unexpired, and marks it used, so a
    /// second use finds nothing.
    ///
    /// # Errors
    ///
    /// Returns [`LinkStoreError`] when the store cannot be written.
    fn redeem(
        &self,
        hash: SessionHash,
        at: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<Option<AccountId>, LinkStoreError>>;
}

/// A sign-in link could not be mailed.
#[derive(Debug, thiserror::Error)]
#[error("the sign-in link could not be sent")]
pub struct SendLinkError {
    #[source]
    source: Box<dyn Error + Send + Sync>,
}

impl SendLinkError {
    /// Wraps what the mail sender reported.
    pub fn new(source: impl Error + Send + Sync + 'static) -> Self {
        Self {
            source: Box::new(source),
        }
    }
}

/// The sign-in links could not be read or written.
#[derive(Debug, thiserror::Error)]
#[error("the sign-in links could not be read or written")]
pub struct LinkStoreError {
    #[source]
    source: Box<dyn Error + Send + Sync>,
}

impl LinkStoreError {
    /// Wraps what the storage reported.
    pub fn new(source: impl Error + Send + Sync + 'static) -> Self {
        Self {
            source: Box::new(source),
        }
    }
}
