//! What was raised already, so each problem is sent once while it lasts.

use std::collections::BTreeSet;
use std::error::Error;
use std::pin::Pin;

use chrono::{DateTime, Utc};

use crate::{Alert, Finding, Kind};

/// A boxed future, so [`Raised`] can be used as `dyn Raised`.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Where the keys of raised findings are kept: a table in `khalid-db` in
/// the binary, [`MemoryRaised`] in tests.
pub trait Raised: Send + Sync {
    /// Every key of `kind` raised and not forgotten.
    ///
    /// # Errors
    ///
    /// [`RaisedError`] when the keys cannot be read.
    fn raised(&self, kind: Kind) -> BoxFuture<'_, Result<BTreeSet<String>, RaisedError>>;

    /// Keeps `keys` as raised at `at`.
    ///
    /// # Errors
    ///
    /// [`RaisedError`] when the keys cannot be written.
    fn remember<'a>(
        &'a self,
        keys: &'a [String],
        at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<(), RaisedError>>;

    /// Drops `keys`, so each is raised again if it comes back.
    ///
    /// # Errors
    ///
    /// [`RaisedError`] when the keys cannot be written.
    fn forget<'a>(&'a self, keys: &'a [String]) -> BoxFuture<'a, Result<(), RaisedError>>;

    /// Drops every key raised before `before`, so the store stays small.
    ///
    /// # Errors
    ///
    /// [`RaisedError`] when the keys cannot be written.
    fn forget_before(&self, before: DateTime<Utc>) -> BoxFuture<'_, Result<(), RaisedError>>;
}

/// The raised keys could not be read or written.
#[derive(Debug, thiserror::Error)]
#[error("the alerts already raised could not be read or written")]
pub struct RaisedError {
    #[source]
    source: Box<dyn Error + Send + Sync>,
}

impl RaisedError {
    /// Wraps the store's own `source`.
    pub fn new(source: impl Error + Send + Sync + 'static) -> Self {
        Self {
            source: Box::new(source),
        }
    }
}

/// The alert of `kind` for the `findings` not raised before, at `at`, or
/// `None` when every one was. Its keys are kept before it is sent, so a
/// failed send is not tried again; the webhook log retries its own.
///
/// When `kind` [clears](Kind::clears), `findings` is the whole picture, so
/// a key of `kind` no longer found is forgotten and raised again if it
/// comes back.
///
/// # Errors
///
/// [`RaisedError`] when `store` cannot be read or written.
pub async fn fresh(
    store: &dyn Raised,
    kind: Kind,
    findings: Vec<Finding>,
    at: DateTime<Utc>,
) -> Result<Option<Alert>, RaisedError> {
    let before = store.raised(kind).await?;
    if kind.clears() {
        let found: BTreeSet<&String> = findings.iter().map(|finding| &finding.key).collect();
        let gone: Vec<String> = before
            .iter()
            .filter(|key| !found.contains(key))
            .cloned()
            .collect();
        if !gone.is_empty() {
            store.forget(&gone).await?;
        }
    }
    let new: Vec<Finding> = findings
        .into_iter()
        .filter(|finding| !before.contains(&finding.key))
        .collect();
    let keys: Vec<String> = new.iter().map(|finding| finding.key.clone()).collect();
    if !keys.is_empty() {
        store.remember(&keys, at).await?;
    }
    Ok(Alert::new(kind, new, at))
}

#[cfg(any(test, feature = "test-util"))]
pub use memory::MemoryRaised;

#[cfg(any(test, feature = "test-util"))]
mod memory {
    use std::collections::BTreeMap;
    use std::sync::{Mutex, PoisonError};

    use super::{BTreeSet, BoxFuture, DateTime, Kind, Raised, RaisedError, Utc};

    /// Raised keys in memory, for tests.
    #[derive(Debug, Default)]
    pub struct MemoryRaised(Mutex<BTreeMap<String, DateTime<Utc>>>);

    impl MemoryRaised {
        fn keys(&self) -> std::sync::MutexGuard<'_, BTreeMap<String, DateTime<Utc>>> {
            self.0.lock().unwrap_or_else(PoisonError::into_inner)
        }
    }

    impl Raised for MemoryRaised {
        fn raised(&self, kind: Kind) -> BoxFuture<'_, Result<BTreeSet<String>, RaisedError>> {
            let keys = self
                .keys()
                .keys()
                .filter(|key| kind.owns(key))
                .cloned()
                .collect();
            Box::pin(async move { Ok(keys) })
        }

        fn remember<'a>(
            &'a self,
            keys: &'a [String],
            at: DateTime<Utc>,
        ) -> BoxFuture<'a, Result<(), RaisedError>> {
            self.keys().extend(keys.iter().map(|key| (key.clone(), at)));
            Box::pin(async { Ok(()) })
        }

        fn forget<'a>(&'a self, keys: &'a [String]) -> BoxFuture<'a, Result<(), RaisedError>> {
            let mut held = self.keys();
            for key in keys {
                held.remove(key);
            }
            Box::pin(async { Ok(()) })
        }

        fn forget_before(&self, before: DateTime<Utc>) -> BoxFuture<'_, Result<(), RaisedError>> {
            self.keys().retain(|_, at| *at >= before);
            Box::pin(async { Ok(()) })
        }
    }
}
