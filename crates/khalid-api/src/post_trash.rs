//! The trash: a deleted post is kept until someone restores it or
//! deletes it for good (KH-EDT-6).

use chrono::{DateTime, Utc};

use crate::content_store::BoxFuture;
use crate::post_store::PostStoreError;
use crate::resources::{AdminPost, PostStatus};

/// A post in the trash.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Trashed {
    /// Names this copy in the trash: when it was deleted, then the post's
    /// identity, such as `20261031T093000Z-en.hello`.
    pub key: String,
    /// The post's identity when it was deleted.
    pub id: String,
    /// Its title.
    pub title: String,
    /// Its status when it was deleted.
    pub status: PostStatus,
    /// When it was deleted.
    pub trashed_at: DateTime<Utc>,
}

impl Trashed {
    /// The copy of `post` deleted at `at`.
    pub fn new(post: &AdminPost, at: DateTime<Utc>) -> Self {
        Self::listed(
            &key(at, post.id()),
            post.id(),
            post.title(),
            post.status(),
            at,
        )
    }

    /// A copy read back from the trash.
    pub fn listed(
        key: &str,
        id: &str,
        title: &str,
        status: PostStatus,
        trashed_at: DateTime<Utc>,
    ) -> Self {
        Self {
            key: key.to_owned(),
            id: id.to_owned(),
            title: title.to_owned(),
            status,
            trashed_at,
        }
    }
}

/// The key of post `id`'s copy deleted at `at`: the time to the second,
/// then the identity, so keys sort by when the post was deleted.
pub fn key(at: DateTime<Utc>, id: &str) -> String {
    format!("{}-{id}", at.format("%Y%m%dT%H%M%SZ"))
}

/// Where deleted posts wait. `khalid serve` keeps them as files under
/// `trash/`; `MemoryPostStore`, from the `test-util` feature, keeps the
/// posts it deletes.
pub trait PostTrash: Send + Sync {
    /// Every post in the trash, the most recently deleted first.
    ///
    /// # Errors
    ///
    /// Returns [`PostStoreError`] when the trash cannot be read.
    fn trashed(&self) -> BoxFuture<'_, Result<Vec<Trashed>, PostStoreError>>;

    /// Puts the copy at `key` back where it was, in the status it had, and
    /// answers the post as it is now.
    ///
    /// # Errors
    ///
    /// Returns [`TrashError::NotFound`] when nothing in the trash has that
    /// key, [`TrashError::Taken`] when a post has its address now, and
    /// [`TrashError::Store`] when the posts cannot be read or written.
    fn restore<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<AdminPost, TrashError>>;

    /// Deletes the copy at `key` for good.
    ///
    /// # Errors
    ///
    /// Returns [`TrashError::NotFound`] when nothing in the trash has that
    /// key, and [`TrashError::Store`] when the trash cannot be written.
    fn purge<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<(), TrashError>>;
}

/// Why a post in the trash was not restored or deleted.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TrashError {
    /// Nothing in the trash has that key.
    #[error("nothing in the trash has that key")]
    NotFound,
    /// Another post has the address the restored one would take.
    #[error("post {id} has the address the restored post would take")]
    Taken {
        /// The identity both posts share.
        id: String,
    },
    /// The posts could not be read or written.
    #[error(transparent)]
    Store(#[from] PostStoreError),
}
