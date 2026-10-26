//! Where the Admin API reads and writes posts.

use std::error::Error;

use crate::content_store::BoxFuture;
use crate::resources::AdminPost;

/// Posts in every status, for the Admin API. Task 1.15 implements it on
/// the content files and the index; `MemoryPostStore`, from the
/// `test-util` feature, keeps posts in memory for tests.
pub trait PostStore: Send + Sync {
    /// The post with identity `id`, in any status, or `None`.
    ///
    /// # Errors
    ///
    /// Returns [`PostStoreError`] when the posts cannot be read.
    fn get<'a>(&'a self, id: &'a str) -> BoxFuture<'a, Result<Option<AdminPost>, PostStoreError>>;
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
    use super::{AdminPost, BoxFuture, PostStore, PostStoreError};

    /// Posts held in memory, for tests.
    #[derive(Debug, Default)]
    pub struct MemoryPostStore {
        posts: Vec<AdminPost>,
    }

    impl MemoryPostStore {
        /// A store holding `posts`.
        pub fn new(posts: Vec<AdminPost>) -> Self {
            Self { posts }
        }
    }

    /// Memory answers at once, so each future is ready when returned.
    impl PostStore for MemoryPostStore {
        fn get<'a>(
            &'a self,
            id: &'a str,
        ) -> BoxFuture<'a, Result<Option<AdminPost>, PostStoreError>> {
            let found = self.posts.iter().find(|post| post.id() == id).cloned();
            Box::pin(std::future::ready(Ok(found)))
        }
    }
}
