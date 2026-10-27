//! Where redirects people make, imports bring and slug changes add are
//! kept (KH-SEO-8, ADR 0010).

use std::error::Error;

use chrono::{DateTime, Utc};
use khalid_seo::RedirectEntry;

use crate::content_store::BoxFuture;

/// Who made a stored redirect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Origin {
    /// A person, through the Admin API or the command line.
    Manual,
    /// A CSV import.
    Import,
    /// Khalid, when a post's address changed.
    SlugChange,
}

impl Origin {
    /// The name stored and shown, such as `slug_change`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Import => "import",
            Self::SlugChange => "slug_change",
        }
    }

    /// The origin named `name`, as [`Origin::as_str`] writes it.
    pub fn from_name(name: &str) -> Option<Self> {
        [Self::Manual, Self::Import, Self::SlugChange]
            .into_iter()
            .find(|origin| origin.as_str() == name)
    }
}

/// A redirect as stored, with its identity, origin and creation time.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct StoredRedirect {
    /// The identity the store gave it.
    pub id: i64,
    /// The rule.
    pub entry: RedirectEntry,
    /// Who made it.
    pub origin: Origin,
    /// When it was made.
    pub created_at: DateTime<Utc>,
}

impl StoredRedirect {
    /// A stored redirect, as a store reads it back.
    pub fn new(id: i64, entry: RedirectEntry, origin: Origin, created_at: DateTime<Utc>) -> Self {
        Self {
            id,
            entry,
            origin,
            created_at,
        }
    }
}

/// The site's stored redirects. `khalid-db` keeps them in
/// SQLite as visitor data; `MemoryRedirectStore`, from the `test-util`
/// feature, keeps them in memory for tests. Checking a new rule against
/// the others for loops is the caller's work; a store only refuses an
/// address that already has a rule.
pub trait RedirectStore: Send + Sync {
    /// Every stored redirect, oldest first.
    ///
    /// # Errors
    ///
    /// [`RedirectStoreError::Storage`] when they cannot be read.
    fn list(&self) -> BoxFuture<'_, Result<Vec<StoredRedirect>, RedirectStoreError>>;

    /// Stores every one of `entries`, made by `origin` at `at`, or none.
    ///
    /// # Errors
    ///
    /// [`RedirectStoreError::Taken`] when an address already has a rule or
    /// appears twice in `entries`, and [`RedirectStoreError::Storage`] when
    /// they cannot be written. Either way nothing is stored.
    fn add(
        &self,
        entries: Vec<RedirectEntry>,
        origin: Origin,
        at: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<Vec<StoredRedirect>, RedirectStoreError>>;

    /// Removes redirect `id`, answering whether it was there.
    ///
    /// # Errors
    ///
    /// [`RedirectStoreError::Storage`] when it cannot be removed.
    fn remove(&self, id: i64) -> BoxFuture<'_, Result<bool, RedirectStoreError>>;
}

/// Why a store refused or failed.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RedirectStoreError {
    /// The address already has a rule.
    #[error("the address {from:?} already has a redirect")]
    Taken {
        /// The address, or the pattern as written.
        from: String,
    },
    /// The storage failed.
    #[error("the redirects could not be read or written")]
    Storage(#[source] Box<dyn Error + Send + Sync>),
}

impl RedirectStoreError {
    /// Wraps what the storage reported.
    pub fn storage(source: impl Error + Send + Sync + 'static) -> Self {
        Self::Storage(Box::new(source))
    }
}

/// The text a rule starts from: its path, or its pattern as written. A
/// store keeps one rule per text.
pub fn source_of(entry: &RedirectEntry) -> &str {
    match entry {
        RedirectEntry::Exact(rule) => rule.from.as_str(),
        RedirectEntry::Pattern(rule) => rule.pattern.as_str(),
        _ => "",
    }
}

#[cfg(any(test, feature = "test-util"))]
pub use memory::MemoryRedirectStore;

#[cfg(any(test, feature = "test-util"))]
mod memory {
    use std::collections::BTreeSet;
    use std::sync::{Mutex, PoisonError};

    use chrono::{DateTime, Utc};
    use khalid_seo::RedirectEntry;

    use super::{BoxFuture, Origin, RedirectStore, RedirectStoreError, StoredRedirect, source_of};

    /// Redirects held in memory, for tests.
    #[derive(Debug, Default)]
    pub struct MemoryRedirectStore(Mutex<State>);

    #[derive(Debug, Default)]
    struct State {
        last_id: i64,
        redirects: Vec<StoredRedirect>,
    }

    impl MemoryRedirectStore {
        fn state(&self) -> std::sync::MutexGuard<'_, State> {
            self.0.lock().unwrap_or_else(PoisonError::into_inner)
        }

        fn add_now(
            &self,
            entries: Vec<RedirectEntry>,
            origin: Origin,
            at: DateTime<Utc>,
        ) -> Result<Vec<StoredRedirect>, RedirectStoreError> {
            let mut state = self.state();
            let mut taken: BTreeSet<String> = state
                .redirects
                .iter()
                .map(|r| source_of(&r.entry).to_owned())
                .collect();
            for entry in &entries {
                if !taken.insert(source_of(entry).to_owned()) {
                    let from = source_of(entry).to_owned();
                    return Err(RedirectStoreError::Taken { from });
                }
            }
            let mut added = Vec::new();
            for entry in entries {
                state.last_id += 1;
                added.push(StoredRedirect::new(state.last_id, entry, origin, at));
            }
            state.redirects.extend(added.iter().cloned());
            Ok(added)
        }
    }

    /// Memory answers at once, so each future is ready when returned.
    impl RedirectStore for MemoryRedirectStore {
        fn list(&self) -> BoxFuture<'_, Result<Vec<StoredRedirect>, RedirectStoreError>> {
            Box::pin(std::future::ready(Ok(self.state().redirects.clone())))
        }

        fn add(
            &self,
            entries: Vec<RedirectEntry>,
            origin: Origin,
            at: DateTime<Utc>,
        ) -> BoxFuture<'_, Result<Vec<StoredRedirect>, RedirectStoreError>> {
            Box::pin(std::future::ready(self.add_now(entries, origin, at)))
        }

        fn remove(&self, id: i64) -> BoxFuture<'_, Result<bool, RedirectStoreError>> {
            let mut state = self.state();
            let before = state.redirects.len();
            state.redirects.retain(|r| r.id != id);
            let removed = state.redirects.len() < before;
            Box::pin(std::future::ready(Ok(removed)))
        }
    }
}
