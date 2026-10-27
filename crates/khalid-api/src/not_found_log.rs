//! The log of addresses that answered 404, so a person can see what readers
//! and search engines asked for and redirect it in one step (KH-SEO-8).

use std::error::Error;

use chrono::{DateTime, Utc};

use crate::content_store::BoxFuture;

/// The most addresses the log holds. Once full, addresses already in it
/// are still counted and new ones are dropped, so a scanner trying random
/// addresses cannot grow the database without end.
pub const MAX_MISSES: usize = 10_000;

/// An address that answered 404, with how often and when.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Miss {
    /// The address as requested, percent-encoding undone.
    pub path: String,
    /// How many requests it answered.
    pub hits: u64,
    /// The first request.
    pub first_seen: DateTime<Utc>,
    /// The latest request.
    pub last_seen: DateTime<Utc>,
}

impl Miss {
    /// A logged address, as a log reads it back.
    pub fn new(path: &str, hits: u64, first_seen: DateTime<Utc>, last_seen: DateTime<Utc>) -> Self {
        Self {
            path: path.to_owned(),
            hits,
            first_seen,
            last_seen,
        }
    }
}

/// The site's 404 log. `khalid-db` keeps it in SQLite as visitor data;
/// `MemoryNotFoundLog`, from the `test-util` feature, keeps it in memory
/// for tests.
pub trait NotFoundLog: Send + Sync {
    /// Counts one 404 for `path` at `at`, up to [`MAX_MISSES`] addresses.
    ///
    /// # Errors
    ///
    /// [`NotFoundLogError`] when the log cannot be written.
    fn record<'a>(
        &'a self,
        path: &'a str,
        at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<(), NotFoundLogError>>;

    /// Up to `limit` logged addresses, most hit first, then by address.
    ///
    /// # Errors
    ///
    /// [`NotFoundLogError`] when the log cannot be read.
    fn misses(&self, limit: u32) -> BoxFuture<'_, Result<Vec<Miss>, NotFoundLogError>>;

    /// Drops `path` from the log, answering whether it was there.
    ///
    /// # Errors
    ///
    /// [`NotFoundLogError`] when the log cannot be written.
    fn forget<'a>(&'a self, path: &'a str) -> BoxFuture<'a, Result<bool, NotFoundLogError>>;
}

/// The 404 log could not be read or written.
#[derive(Debug, thiserror::Error)]
#[error("the log of addresses not found could not be read or written")]
pub struct NotFoundLogError {
    #[source]
    source: Box<dyn Error + Send + Sync>,
}

impl NotFoundLogError {
    /// Wraps what the storage reported.
    pub fn new(source: impl Error + Send + Sync + 'static) -> Self {
        Self {
            source: Box::new(source),
        }
    }
}

#[cfg(any(test, feature = "test-util"))]
pub use memory::MemoryNotFoundLog;

#[cfg(any(test, feature = "test-util"))]
mod memory {
    use std::collections::BTreeMap;
    use std::sync::{Mutex, PoisonError};

    use chrono::{DateTime, Utc};

    use super::{BoxFuture, MAX_MISSES, Miss, NotFoundLog, NotFoundLogError};

    /// A 404 log held in memory, for tests.
    #[derive(Debug, Default)]
    pub struct MemoryNotFoundLog(Mutex<BTreeMap<String, Miss>>);

    impl MemoryNotFoundLog {
        fn record_now(&self, path: &str, at: DateTime<Utc>) {
            let mut misses = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            let full = misses.len() >= MAX_MISSES;
            match misses.get_mut(path) {
                Some(miss) => {
                    miss.hits += 1;
                    miss.last_seen = at;
                }
                None if full => {}
                None => {
                    misses.insert(path.to_owned(), Miss::new(path, 1, at, at));
                }
            }
        }

        fn misses_now(&self, limit: u32) -> Vec<Miss> {
            let misses = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            let mut misses: Vec<Miss> = misses.values().cloned().collect();
            misses.sort_by(|a, b| b.hits.cmp(&a.hits).then_with(|| a.path.cmp(&b.path)));
            misses.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
            misses
        }
    }

    /// Memory answers at once, so each future is ready when returned.
    impl NotFoundLog for MemoryNotFoundLog {
        fn record<'a>(
            &'a self,
            path: &'a str,
            at: DateTime<Utc>,
        ) -> BoxFuture<'a, Result<(), NotFoundLogError>> {
            self.record_now(path, at);
            Box::pin(std::future::ready(Ok(())))
        }

        fn misses(&self, limit: u32) -> BoxFuture<'_, Result<Vec<Miss>, NotFoundLogError>> {
            Box::pin(std::future::ready(Ok(self.misses_now(limit))))
        }

        fn forget<'a>(&'a self, path: &'a str) -> BoxFuture<'a, Result<bool, NotFoundLogError>> {
            let mut misses = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            let forgotten = misses.remove(path).is_some();
            Box::pin(std::future::ready(Ok(forgotten)))
        }
    }
}
