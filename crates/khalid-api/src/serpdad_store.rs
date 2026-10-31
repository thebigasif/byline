//! Where the `SerpDad` connection is kept: which key `SerpDad` sends drafts
//! with, and the drafts that arrived with it, for the Connect to `SerpDad`
//! page (KH-API-6, `docs/decisions/2026-10-31-connect-to-serpdad.md`).

use std::error::Error;

use chrono::{DateTime, Utc};

use crate::content_store::BoxFuture;
use crate::keys::KeyId;
use crate::resources::LintCounts;

/// How many received drafts the page lists.
pub const RECENT_DRAFTS: u32 = 20;

/// The key `SerpDad` sends drafts with, and since when.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SerpdadLink {
    /// The key's identity; the key itself is never kept.
    pub key: KeyId,
    /// When the key was made.
    pub connected_at: DateTime<Utc>,
}

impl SerpdadLink {
    /// The link to `key`, made at `connected_at`.
    pub fn new(key: KeyId, connected_at: DateTime<Utc>) -> Self {
        Self { key, connected_at }
    }
}

/// A draft `SerpDad` sent, as it arrived.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ArrivedDraft {
    /// The post's identity, such as `en.choosing-anchor-text`.
    pub post: String,
    /// Its title as it arrived.
    pub title: String,
    /// What the linter found in it, or `None` when it could not run.
    pub lint: Option<LintCounts>,
}

impl ArrivedDraft {
    /// Draft `post` titled `title`, with the linter's counts `lint`.
    pub fn new(post: String, title: String, lint: Option<LintCounts>) -> Self {
        Self { post, title, lint }
    }
}

/// A recorded draft and when it arrived.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ReceivedDraft {
    /// The draft.
    pub draft: ArrivedDraft,
    /// When it arrived.
    pub received_at: DateTime<Utc>,
}

impl ReceivedDraft {
    /// `draft`, which arrived at `received_at`.
    pub fn new(draft: ArrivedDraft, received_at: DateTime<Utc>) -> Self {
        Self { draft, received_at }
    }
}

/// The site's `SerpDad` connection. `khalid-db` keeps it in SQLite as
/// visitor data; `MemorySerpdadStore`, from the `test-util` feature, keeps
/// it in memory for tests.
pub trait SerpdadStore: Send + Sync {
    /// The key `SerpDad` sends drafts with, or `None` while the connection
    /// is off.
    ///
    /// # Errors
    ///
    /// [`SerpdadStoreError`] when it cannot be read.
    fn link(&self) -> BoxFuture<'_, Result<Option<SerpdadLink>, SerpdadStoreError>>;

    /// Makes `key` the one `SerpDad` sends drafts with, and answers the key
    /// it replaces, which the caller revokes.
    ///
    /// # Errors
    ///
    /// [`SerpdadStoreError`] when it cannot be written.
    fn connect<'a>(
        &'a self,
        key: &'a KeyId,
    ) -> BoxFuture<'a, Result<Option<KeyId>, SerpdadStoreError>>;

    /// Turns the connection off, and answers the key it had, which the
    /// caller revokes. The received drafts stay.
    ///
    /// # Errors
    ///
    /// [`SerpdadStoreError`] when it cannot be written.
    fn disconnect(&self) -> BoxFuture<'_, Result<Option<KeyId>, SerpdadStoreError>>;

    /// Records `draft` as arrived now.
    ///
    /// # Errors
    ///
    /// [`SerpdadStoreError`] when it cannot be written.
    fn record<'a>(
        &'a self,
        draft: &'a ArrivedDraft,
    ) -> BoxFuture<'a, Result<(), SerpdadStoreError>>;

    /// The last [`RECENT_DRAFTS`] drafts that arrived, newest first.
    ///
    /// # Errors
    ///
    /// [`SerpdadStoreError`] when they cannot be read.
    fn recent(&self) -> BoxFuture<'_, Result<Vec<ReceivedDraft>, SerpdadStoreError>>;
}

/// The connection could not be read or written.
#[derive(Debug, thiserror::Error)]
#[error("the SerpDad connection could not be read or written")]
pub struct SerpdadStoreError(#[source] Box<dyn Error + Send + Sync>);

impl SerpdadStoreError {
    /// Wraps what the storage reported.
    pub fn storage(source: impl Error + Send + Sync + 'static) -> Self {
        Self(Box::new(source))
    }
}

#[cfg(any(test, feature = "test-util"))]
pub use memory::MemorySerpdadStore;

#[cfg(any(test, feature = "test-util"))]
mod memory {
    use std::sync::{Mutex, MutexGuard, PoisonError};

    use chrono::{DateTime, Utc};

    use super::{
        ArrivedDraft, BoxFuture, KeyId, RECENT_DRAFTS, ReceivedDraft, SerpdadLink, SerpdadStore,
        SerpdadStoreError,
    };

    /// The link and the drafts, newest last.
    type Held = (Option<SerpdadLink>, Vec<ReceivedDraft>);

    /// A connection held in memory, for tests, with every time `at`.
    #[derive(Debug)]
    pub struct MemorySerpdadStore {
        at: DateTime<Utc>,
        held: Mutex<Held>,
    }

    impl MemorySerpdadStore {
        /// Off, with no drafts; everything after is dated `at`.
        pub fn new(at: DateTime<Utc>) -> Self {
            Self {
                at,
                held: Mutex::default(),
            }
        }

        fn held(&self) -> MutexGuard<'_, Held> {
            self.held.lock().unwrap_or_else(PoisonError::into_inner)
        }
    }

    /// Memory answers at once, so each future is ready when returned.
    impl SerpdadStore for MemorySerpdadStore {
        fn link(&self) -> BoxFuture<'_, Result<Option<SerpdadLink>, SerpdadStoreError>> {
            Box::pin(std::future::ready(Ok(self.held().0.clone())))
        }

        fn connect<'a>(
            &'a self,
            key: &'a KeyId,
        ) -> BoxFuture<'a, Result<Option<KeyId>, SerpdadStoreError>> {
            let link = SerpdadLink::new(key.clone(), self.at);
            let old = self.held().0.replace(link).map(|old| old.key);
            Box::pin(std::future::ready(Ok(old)))
        }

        fn disconnect(&self) -> BoxFuture<'_, Result<Option<KeyId>, SerpdadStoreError>> {
            let old = self.held().0.take().map(|old| old.key);
            Box::pin(std::future::ready(Ok(old)))
        }

        fn record<'a>(
            &'a self,
            draft: &'a ArrivedDraft,
        ) -> BoxFuture<'a, Result<(), SerpdadStoreError>> {
            let received = ReceivedDraft::new(draft.clone(), self.at);
            self.held().1.push(received);
            Box::pin(std::future::ready(Ok(())))
        }

        fn recent(&self) -> BoxFuture<'_, Result<Vec<ReceivedDraft>, SerpdadStoreError>> {
            let held = self.held();
            let newest = held
                .1
                .iter()
                .rev()
                .take(RECENT_DRAFTS.try_into().unwrap_or(usize::MAX));
            Box::pin(std::future::ready(Ok(newest.cloned().collect())))
        }
    }
}
