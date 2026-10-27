//! Where comments are kept.

use std::error::Error;
use std::future::Future;

use chrono::{DateTime, Utc};

use crate::{Comment, CommentAuthor, CommentId, CommentStatus, CommentText, PostFile};

/// A comment to store, before it has an identity, a status or a time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewComment {
    /// The post it is on.
    pub post: PostFile,
    /// The comment it answers.
    pub parent: Option<CommentId>,
    /// Who wrote it.
    pub author: CommentAuthor,
    /// What they wrote.
    pub text: CommentText,
}

/// The comments' storage. `khalid-db` implements it on SQLite, and
/// `MemoryCommentStore`, from the `test-util` feature, keeps it in memory
/// for tests.
pub trait CommentStore: Send + Sync {
    /// Stores `new` with `status`, sent `at`, and returns it with its identity.
    ///
    /// # Errors
    ///
    /// [`CommentStoreError::Write`] when it cannot be written.
    fn insert(
        &self,
        new: NewComment,
        status: CommentStatus,
        at: DateTime<Utc>,
    ) -> impl Future<Output = Result<Comment, CommentStoreError>> + Send;

    /// The comment `id`, or `None`.
    ///
    /// # Errors
    ///
    /// [`CommentStoreError::Read`] when the comments cannot be read.
    fn get(
        &self,
        id: CommentId,
    ) -> impl Future<Output = Result<Option<Comment>, CommentStoreError>> + Send;

    /// Gives comment `id` the `status`, and returns it as it is now, or
    /// `None` when there is no such comment.
    ///
    /// # Errors
    ///
    /// [`CommentStoreError::Write`] when it cannot be written.
    fn set_status(
        &self,
        id: CommentId,
        status: CommentStatus,
    ) -> impl Future<Output = Result<Option<Comment>, CommentStoreError>> + Send;

    /// Removes comment `id` and every reply to it, and returns it as it
    /// was, or `None` when there is no such comment.
    ///
    /// # Errors
    ///
    /// [`CommentStoreError::Write`] when it cannot be written.
    fn delete(
        &self,
        id: CommentId,
    ) -> impl Future<Output = Result<Option<Comment>, CommentStoreError>> + Send;

    /// Every comment with `status`, on `post` or on every post when it is
    /// `None`, oldest first.
    ///
    /// # Errors
    ///
    /// [`CommentStoreError::Read`] when the comments cannot be read.
    fn with_status(
        &self,
        status: CommentStatus,
        post: Option<&PostFile>,
    ) -> impl Future<Output = Result<Vec<Comment>, CommentStoreError>> + Send;

    /// The newest `limit` comments with `status`, or with any status when
    /// it is `None`, newest first.
    ///
    /// # Errors
    ///
    /// [`CommentStoreError::Read`] when the comments cannot be read.
    fn newest(
        &self,
        status: Option<CommentStatus>,
        limit: u32,
    ) -> impl Future<Output = Result<Vec<Comment>, CommentStoreError>> + Send;
}

/// The comments could not be read or written.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CommentStoreError {
    /// Reading failed.
    #[error("the comments could not be read")]
    Read {
        /// What the storage reported.
        #[source]
        source: Box<dyn Error + Send + Sync>,
    },
    /// Writing failed.
    #[error("the comments could not be written")]
    Write {
        /// What the storage reported.
        #[source]
        source: Box<dyn Error + Send + Sync>,
    },
}
