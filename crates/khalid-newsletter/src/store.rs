//! Where subscribers are kept: the trait `khalid-db` implements, and the
//! answers it gives.

use std::error::Error;
use std::fmt;
use std::future::Future;
use std::pin::Pin;

use chrono::{DateTime, TimeDelta, Utc};
use khalid_core::Slug;

use crate::{EmailAddress, TokenHash};

/// How long a confirmation link works. Two days covers a weekend away from
/// the inbox, and a reader who misses it signs up again.
const CONFIRM_FOR: TimeDelta = TimeDelta::days(2);

/// How long after one confirmation email another one is held back, so a
/// form sent over and over cannot fill someone's inbox.
const QUIET_FOR: TimeDelta = TimeDelta::minutes(10);

/// A future the store returns, boxed so the trait can be a `dyn` object.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A subscriber's row, as the store numbers it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubscriberId(i64);

impl SubscriberId {
    /// The subscriber with row number `row`.
    pub fn from_row(row: i64) -> Self {
        Self(row)
    }

    /// The row number to bind in a query.
    pub fn as_row(self) -> i64 {
        self.0
    }
}

/// One sign-up from the form: who, to which newsletter, and the hash of the
/// confirmation token to store if a confirmation email is due.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SignUp {
    /// The address typed into the form.
    pub email: EmailAddress,
    /// The newsletter they asked for.
    pub list: Slug,
    /// The hash of the token in the confirmation link.
    pub confirm: TokenHash,
    /// When the form arrived.
    pub at: DateTime<Utc>,
    /// When the confirmation link stops working.
    pub expires_at: DateTime<Utc>,
    /// A confirmation sent after this time is still fresh, so no new one
    /// is sent.
    pub quiet_since: DateTime<Utc>,
}

impl SignUp {
    /// A sign-up of `email` to `list` that arrived `at`, whose confirmation
    /// link, if one is sent, holds the token hashed as `confirm`.
    pub fn new(email: EmailAddress, list: Slug, confirm: TokenHash, at: DateTime<Utc>) -> Self {
        Self {
            email,
            list,
            confirm,
            at,
            expires_at: at + CONFIRM_FOR,
            quiet_since: at - QUIET_FOR,
        }
    }
}

/// What the store did with a [`SignUp`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SignedUp {
    /// The address is waiting for its owner to confirm, and the token was
    /// stored: send the confirmation email now.
    ConfirmationDue(SubscriberId),
    /// A confirmation email went out a moment ago. Nothing was stored, and
    /// nothing is sent.
    RecentlyAsked,
    /// The address was confirmed before. It now gets this newsletter too,
    /// and nothing is sent.
    AlreadyConfirmed,
}

/// What a confirmation link did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Confirmed {
    /// The subscriber is confirmed from now on.
    Now(SubscriberId),
    /// The link was opened before, and the subscriber is still confirmed.
    Before,
    /// The link is older than two days.
    Expired,
    /// No link was ever sent with this token, or its subscriber has left.
    Unknown,
}

/// One site's subscribers. Every time comes from the caller's clock, and
/// every token is a hash (RULES.md BL-RUN-1, BL-SEC-3).
pub trait SubscriberStore: fmt::Debug + Send + Sync {
    /// Records `sign_up`, and stores its confirmation token unless the
    /// address is confirmed already or was asked a moment ago.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when the store cannot be read or written.
    fn sign_up<'a>(&'a self, sign_up: &'a SignUp) -> BoxFuture<'a, Result<SignedUp, StoreError>>;

    /// Confirms the subscriber whose confirmation token hashes to `token`,
    /// if the link still works `at` that time. A link works once.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when the store cannot be read or written.
    fn confirm<'a>(
        &'a self,
        token: &'a TokenHash,
        at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<Confirmed, StoreError>>;
}

/// The store could not be read or written. It keeps the store's own error
/// as its cause.
#[derive(Debug, thiserror::Error)]
#[error("the subscriber list could not be read or written")]
pub struct StoreError {
    #[source]
    source: Box<dyn Error + Send + Sync>,
}

impl StoreError {
    /// Wraps the store's own `source`.
    pub fn new(source: impl Error + Send + Sync + 'static) -> Self {
        Self {
            source: Box::new(source),
        }
    }
}
