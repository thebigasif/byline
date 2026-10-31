//! Where the publish door keeps each post's earlier versions: the title and
//! text an edit replaced, so a writer can see what changed and bring an old
//! text back (plan 4.6, KH-EDT-2).

use std::error::Error;

use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

use crate::content_store::BoxFuture;
use crate::resources::AdminPost;

/// How many earlier versions are kept for each post. An older one is
/// dropped when a new one comes.
pub const MAX_VERSIONS: usize = 100;

/// A post's title and text as they were saved, before a later edit
/// replaced them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct PostVersion {
    /// When this title and text were saved.
    saved_at: DateTime<Utc>,
    /// Who saved them, as the post named them; `null` when not known, such
    /// as for a file written by hand.
    saved_by: Option<String>,
    /// The title.
    title: String,
    /// The text, in markdown.
    markdown: String,
}

impl PostVersion {
    /// The title and text of `post` as it is now, saved when it says.
    pub fn of(post: &AdminPost) -> Self {
        Self {
            saved_at: post.updated_at(),
            saved_by: post.updated_by().map(str::to_owned),
            title: post.title().to_owned(),
            markdown: post.markdown().to_owned(),
        }
    }

    /// The version saved at `saved_at` by `saved_by`, as a store reads it
    /// back.
    pub fn new(
        (saved_at, saved_by): (DateTime<Utc>, Option<String>),
        title: impl Into<String>,
        markdown: impl Into<String>,
    ) -> Self {
        Self {
            saved_at,
            saved_by,
            title: title.into(),
            markdown: markdown.into(),
        }
    }

    /// When this title and text were saved.
    pub fn saved_at(&self) -> DateTime<Utc> {
        self.saved_at
    }

    /// Who saved them, if known.
    pub fn saved_by(&self) -> Option<&str> {
        self.saved_by.as_deref()
    }

    /// The title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The text, in markdown.
    pub fn markdown(&self) -> &str {
        &self.markdown
    }

    /// Whether `post` reads differently: another title or another text.
    pub fn differs_from(&self, post: &AdminPost) -> bool {
        self.title != post.title() || self.markdown != post.markdown()
    }
}

/// Each post's earlier versions. `khalid-db` keeps them in SQLite, so they
/// outlast a restart; `MemoryVersionStore`, from the `test-util` feature,
/// keeps them in memory for tests.
pub trait VersionStore: Send + Sync {
    /// Keeps `version` of post `id`, then drops all but the
    /// [`MAX_VERSIONS`] newest.
    ///
    /// # Errors
    ///
    /// Returns [`VersionStoreError`] when the versions cannot be written.
    fn keep<'a>(
        &'a self,
        id: &'a str,
        version: PostVersion,
    ) -> BoxFuture<'a, Result<(), VersionStoreError>>;

    /// The earlier versions of post `id`, the newest first.
    ///
    /// # Errors
    ///
    /// Returns [`VersionStoreError`] when the versions cannot be read.
    fn list<'a>(
        &'a self,
        id: &'a str,
    ) -> BoxFuture<'a, Result<Vec<PostVersion>, VersionStoreError>>;
}

/// The versions could not be read or written.
#[derive(Debug, thiserror::Error)]
#[error("the earlier versions could not be read or written")]
pub struct VersionStoreError {
    #[source]
    source: Box<dyn Error + Send + Sync>,
}

impl VersionStoreError {
    /// Wraps what the storage reported.
    pub fn new(source: impl Error + Send + Sync + 'static) -> Self {
        Self {
            source: Box::new(source),
        }
    }
}

#[cfg(any(test, feature = "test-util"))]
pub use memory::MemoryVersionStore;

#[cfg(any(test, feature = "test-util"))]
mod memory {
    use std::collections::BTreeMap;
    use std::sync::{Mutex, PoisonError};

    use super::{BoxFuture, MAX_VERSIONS, PostVersion, VersionStore, VersionStoreError};

    /// Versions held in memory, for tests, the newest first.
    #[derive(Debug, Default)]
    pub struct MemoryVersionStore(Mutex<BTreeMap<String, Vec<PostVersion>>>);

    /// Memory answers at once, so each future is ready when returned.
    impl VersionStore for MemoryVersionStore {
        fn keep<'a>(
            &'a self,
            id: &'a str,
            version: PostVersion,
        ) -> BoxFuture<'a, Result<(), VersionStoreError>> {
            let mut kept = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            let versions = kept.entry(id.to_owned()).or_default();
            versions.insert(0, version);
            versions.truncate(MAX_VERSIONS);
            Box::pin(std::future::ready(Ok(())))
        }

        fn list<'a>(
            &'a self,
            id: &'a str,
        ) -> BoxFuture<'a, Result<Vec<PostVersion>, VersionStoreError>> {
            let kept = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            let versions = kept.get(id).cloned().unwrap_or_default();
            Box::pin(std::future::ready(Ok(versions)))
        }
    }
}
