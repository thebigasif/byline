//! The audit log: who changed what through the Admin API, and when
//! (KH-EDT-6). Every write to the Admin API that succeeds is one change,
//! so content, settings and staff accounts are all covered by one rule.

use std::error::Error;

use axum::http::Method;
use chrono::{DateTime, Utc};

use crate::content_store::BoxFuture;

/// The most changes the log keeps. Past it, the oldest are dropped, so a
/// busy integration key cannot grow the database without end.
pub const MAX_CHANGES: u32 = 100_000;

/// What a change did, from the request's method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Action {
    /// Something new: a `POST`.
    Added,
    /// Something changed: a `PUT` or a `PATCH`.
    Changed,
    /// Something went: a `DELETE`.
    Removed,
}

impl Action {
    /// The action a request with `method` takes, or `None` for a read.
    pub fn of(method: &Method) -> Option<Self> {
        match *method {
            Method::POST => Some(Self::Added),
            Method::PUT | Method::PATCH => Some(Self::Changed),
            Method::DELETE => Some(Self::Removed),
            _ => None,
        }
    }

    /// The action as stored and as the API writes it.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Changed => "changed",
            Self::Removed => "removed",
        }
    }

    /// The action `text` names, as [`Action::as_str`] writes it.
    pub fn parse(text: &str) -> Option<Self> {
        [Self::Added, Self::Changed, Self::Removed]
            .into_iter()
            .find(|action| action.as_str() == text)
    }
}

/// One change made through the Admin API.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Change {
    /// When the change was made.
    pub at: DateTime<Utc>,
    /// Who made it: the identity of the key, or of the staff account.
    pub by: String,
    /// What it did.
    pub action: Action,
    /// The route it went through, such as `/api/admin/v1/posts/{id}/`.
    pub route: String,
    /// The address it was made at, such as `/api/admin/v1/posts/en.hello/`.
    pub path: String,
}

impl Change {
    /// A change `by` made at `at`, through `route` at `path`.
    pub fn new(at: DateTime<Utc>, by: &str, action: Action, route: &str, path: &str) -> Self {
        Self {
            at,
            by: by.to_owned(),
            action,
            route: route.to_owned(),
            path: path.to_owned(),
        }
    }
}

/// A change as the log keeps it, with its place in the log.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Logged {
    /// Its place in the log: a later change has a larger number.
    pub id: i64,
    /// The change.
    pub change: Change,
}

impl Logged {
    /// A change read back from the log at place `id`.
    pub fn new(id: i64, change: Change) -> Self {
        Self { id, change }
    }
}

/// The site's audit log. `khalid-db` keeps it in SQLite as visitor data;
/// `MemoryAuditLog`, from the `test-util` feature, keeps it in memory for
/// tests.
pub trait AuditLog: Send + Sync {
    /// Keeps `change`, dropping the oldest past [`MAX_CHANGES`].
    ///
    /// # Errors
    ///
    /// [`AuditLogError`] when the log cannot be written.
    fn record<'a>(&'a self, change: &'a Change) -> BoxFuture<'a, Result<(), AuditLogError>>;

    /// Up to `limit` changes, newest first, all older than the one at
    /// `before` when given.
    ///
    /// # Errors
    ///
    /// [`AuditLogError`] when the log cannot be read.
    fn changes(
        &self,
        before: Option<i64>,
        limit: u32,
    ) -> BoxFuture<'_, Result<Vec<Logged>, AuditLogError>>;
}

/// The audit log could not be read or written.
#[derive(Debug, thiserror::Error)]
#[error("the audit log could not be read or written")]
pub struct AuditLogError {
    #[source]
    source: Box<dyn Error + Send + Sync>,
}

impl AuditLogError {
    /// Wraps what the storage reported.
    pub fn new(source: impl Error + Send + Sync + 'static) -> Self {
        Self {
            source: Box::new(source),
        }
    }
}

#[cfg(any(test, feature = "test-util"))]
pub use memory::MemoryAuditLog;

#[cfg(any(test, feature = "test-util"))]
mod memory {
    use std::sync::{Mutex, PoisonError};

    use super::{AuditLog, AuditLogError, BoxFuture, Change, Logged, MAX_CHANGES};

    /// An audit log held in memory, for tests.
    #[derive(Debug, Default)]
    pub struct MemoryAuditLog(Mutex<Vec<Logged>>);

    impl MemoryAuditLog {
        fn record_now(&self, change: &Change) {
            let mut log = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            let id = log.last().map_or(1, |last| last.id + 1);
            log.push(Logged::new(id, change.clone()));
            let most = usize::try_from(MAX_CHANGES).unwrap_or(usize::MAX);
            let over = log.len().saturating_sub(most);
            log.drain(..over);
        }

        fn changes_now(&self, before: Option<i64>, limit: u32) -> Vec<Logged> {
            let log = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            log.iter()
                .rev()
                .filter(|logged| before.is_none_or(|before| logged.id < before))
                .take(usize::try_from(limit).unwrap_or(usize::MAX))
                .cloned()
                .collect()
        }
    }

    impl AuditLog for MemoryAuditLog {
        fn record<'a>(&'a self, change: &'a Change) -> BoxFuture<'a, Result<(), AuditLogError>> {
            self.record_now(change);
            Box::pin(async { Ok(()) })
        }

        fn changes(
            &self,
            before: Option<i64>,
            limit: u32,
        ) -> BoxFuture<'_, Result<Vec<Logged>, AuditLogError>> {
            let changes = self.changes_now(before, limit);
            Box::pin(async move { Ok(changes) })
        }
    }
}
