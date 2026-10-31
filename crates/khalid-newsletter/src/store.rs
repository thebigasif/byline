//! Where subscribers are kept: the trait `khalid-db` implements, and the
//! answers it gives.

use std::error::Error;
use std::fmt;
use std::future::Future;
use std::pin::Pin;

use chrono::{DateTime, TimeDelta, Utc};
use khalid_core::Slug;

use crate::{EmailAddress, Labelled, Recipient, Segment, SendId, SendTotals, TokenHash};

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

/// What an unsubscribe link did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Unsubscribed {
    /// The subscriber gets no more email, now or since an earlier click.
    Done,
    /// No email was ever sent with this token.
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

    /// Stores `token` as one more working unsubscribe link of `subscriber`,
    /// for an email about to be sent. It never expires.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when the store cannot be written, or `subscriber` is
    /// gone.
    fn add_unsubscribe<'a>(
        &'a self,
        subscriber: SubscriberId,
        token: &'a TokenHash,
        at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<(), StoreError>>;

    /// Unsubscribes the subscriber whose unsubscribe token hashes to
    /// `token` from every newsletter, `at` that time. Clicking again
    /// changes nothing.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when the store cannot be read or written.
    fn unsubscribe<'a>(
        &'a self,
        token: &'a TokenHash,
        at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<Unsubscribed, StoreError>>;

    /// Gives the subscriber at `email` the label `label`, `at` that time,
    /// unless they unsubscribed. Adding it twice changes nothing.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when the store cannot be read or written.
    fn label<'a>(
        &'a self,
        email: &'a EmailAddress,
        label: &'a Slug,
        at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<Labelled, StoreError>>;

    /// Takes the label `label` away from the subscriber at `email`.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when the store cannot be read or written.
    fn unlabel<'a>(
        &'a self,
        email: &'a EmailAddress,
        label: &'a Slug,
    ) -> BoxFuture<'a, Result<Labelled, StoreError>>;

    /// Records that the issue with `key` starts going out to `list` `at`
    /// that time, or `None` when an issue with that key went out before.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when the store cannot be read or written.
    fn start_send<'a>(
        &'a self,
        key: &'a str,
        list: &'a Slug,
        at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<Option<SendId>, StoreError>>;

    /// Stores `token` as the unsubscribe link of `subscriber` in their
    /// copy of `send`, so a click on it counts against that email.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when the store cannot be written, or `subscriber` is
    /// gone.
    fn add_send_unsubscribe<'a>(
        &'a self,
        send: SendId,
        subscriber: SubscriberId,
        token: &'a TokenHash,
        at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<(), StoreError>>;

    /// Records the `totals` of `send` once it has gone out, `at` that time.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when the store cannot be written.
    fn finish_send(
        &self,
        send: SendId,
        totals: SendTotals,
        at: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<(), StoreError>>;

    /// Every confirmed subscriber in `segment`, oldest first.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when the store cannot be read.
    fn audience<'a>(
        &'a self,
        segment: &'a Segment,
    ) -> BoxFuture<'a, Result<Vec<Recipient>, StoreError>>;
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
