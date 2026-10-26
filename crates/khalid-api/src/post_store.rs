//! Where the Admin API reads and writes posts.

use std::error::Error;

use chrono::{DateTime, Utc};

use crate::content_store::BoxFuture;
use crate::resources::{AdminPost, PostEdit};

/// Posts in every status, for the Admin API. Task 1.15 implements it on
/// the content files and the index, writing each file atomically
/// (RULES.md BL-RUN-9); `MemoryPostStore`, from the `test-util` feature,
/// keeps posts in memory for tests.
pub trait PostStore: Send + Sync {
    /// The post with identity `id`, in any status, or `None`.
    ///
    /// # Errors
    ///
    /// Returns [`PostStoreError`] when the posts cannot be read.
    fn get<'a>(&'a self, id: &'a str) -> BoxFuture<'a, Result<Option<AdminPost>, PostStoreError>>;

    /// Applies `edit`, made by `by`, to post `id`, but only when the post
    /// has not changed since the copy the edit was made on (BL-API-7). The
    /// check and the write happen together.
    ///
    /// # Errors
    ///
    /// Returns [`UpdatePostError::Stale`] naming the last change when the
    /// post changed after that copy, [`UpdatePostError::NotFound`] when no
    /// post has that identity, and [`UpdatePostError::Store`] when the
    /// posts cannot be read or written.
    fn update<'a>(
        &'a self,
        id: &'a str,
        edit: PostEdit,
        by: &'a str,
    ) -> BoxFuture<'a, Result<AdminPost, UpdatePostError>>;
}

/// Why an edit was not saved.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum UpdatePostError {
    /// No post has that identity.
    #[error("no post has that identity")]
    NotFound,
    /// The post changed after the copy the edit was made on.
    #[error("the post changed after the copy the edit was made on")]
    Stale {
        /// When the post last changed.
        updated_at: DateTime<Utc>,
        /// Who changed it, if known.
        updated_by: Option<String>,
    },
    /// The posts could not be read or written.
    #[error(transparent)]
    Store(#[from] PostStoreError),
}

/// The posts could not be read or written.
#[derive(Debug, thiserror::Error)]
#[error("the posts could not be read or written")]
pub struct PostStoreError {
    #[source]
    source: Box<dyn Error + Send + Sync>,
}

impl PostStoreError {
    /// Wraps what the storage reported.
    pub fn new(source: impl Error + Send + Sync + 'static) -> Self {
        Self {
            source: Box::new(source),
        }
    }
}

#[cfg(any(test, feature = "test-util"))]
pub use memory::MemoryPostStore;

#[cfg(any(test, feature = "test-util"))]
mod memory {
    use std::sync::{Mutex, MutexGuard, PoisonError};

    use khalid_core::{Clock, SystemClock};

    use super::{AdminPost, BoxFuture, PostEdit, PostStore, PostStoreError, UpdatePostError};

    /// Posts held in memory, for tests.
    #[derive(Debug)]
    pub struct MemoryPostStore {
        posts: Mutex<Vec<AdminPost>>,
        clock: Box<dyn Clock>,
    }

    impl Default for MemoryPostStore {
        fn default() -> Self {
            Self::new(Vec::new())
        }
    }

    impl MemoryPostStore {
        /// A store holding `posts`, stamping edits with the system clock.
        pub fn new(posts: Vec<AdminPost>) -> Self {
            Self {
                posts: Mutex::new(posts),
                clock: Box::new(SystemClock),
            }
        }

        /// The same store, stamping edits with `clock`.
        #[must_use]
        pub fn with_clock(self, clock: impl Clock + 'static) -> Self {
            Self {
                clock: Box::new(clock),
                ..self
            }
        }

        fn posts(&self) -> MutexGuard<'_, Vec<AdminPost>> {
            self.posts.lock().unwrap_or_else(PoisonError::into_inner)
        }

        fn apply(&self, id: &str, edit: PostEdit, by: &str) -> Result<AdminPost, UpdatePostError> {
            let mut posts = self.posts();
            let post = posts
                .iter_mut()
                .find(|post| post.id() == id)
                .ok_or(UpdatePostError::NotFound)?;
            if post.updated_at() != edit.updated_at() {
                return Err(UpdatePostError::Stale {
                    updated_at: post.updated_at(),
                    updated_by: post.updated_by().map(str::to_owned),
                });
            }
            *post = post.clone().edited(edit, self.clock.now(), by);
            Ok(post.clone())
        }
    }

    /// Memory answers at once, so each future is ready when returned.
    impl PostStore for MemoryPostStore {
        fn get<'a>(
            &'a self,
            id: &'a str,
        ) -> BoxFuture<'a, Result<Option<AdminPost>, PostStoreError>> {
            let found = self.posts().iter().find(|post| post.id() == id).cloned();
            Box::pin(std::future::ready(Ok(found)))
        }

        fn update<'a>(
            &'a self,
            id: &'a str,
            edit: PostEdit,
            by: &'a str,
        ) -> BoxFuture<'a, Result<AdminPost, UpdatePostError>> {
            Box::pin(std::future::ready(self.apply(id, edit, by)))
        }
    }
}
