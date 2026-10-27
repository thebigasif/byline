//! Where the Admin API moderates comments.

use std::error::Error;

use crate::content_store::BoxFuture;
use crate::resources::{AdminComment, CommentReply, CommentStatus};

/// Readers' comments, for the Admin API. `khalid-comments` implements it
/// over its one set of moderation rules; [`NoComments`] stands in for a
/// site that keeps none.
pub trait CommentModeration: Send + Sync {
    /// The newest `limit` comments with `status`, or with any status,
    /// newest first.
    ///
    /// # Errors
    ///
    /// [`ModerationError::Store`] when the comments cannot be read.
    fn list(
        &self,
        status: Option<CommentStatus>,
        limit: u32,
    ) -> BoxFuture<'_, Result<Vec<AdminComment>, ModerationError>>;

    /// Shows comment `id` under its post, and returns it.
    ///
    /// # Errors
    ///
    /// [`ModerationError::NotFound`] when there is no such comment, and
    /// [`ModerationError::Store`] when it cannot be written.
    fn approve(&self, id: i64) -> BoxFuture<'_, Result<AdminComment, ModerationError>>;

    /// Answers comment `id` with `reply`, shown at once, and returns the
    /// reply. The comment answered is approved too.
    ///
    /// # Errors
    ///
    /// [`ModerationError::NotFound`] when there is no such comment,
    /// [`ModerationError::Invalid`] when the reply's name or text cannot
    /// be kept, and [`ModerationError::Store`] when it cannot be written.
    fn reply(
        &self,
        id: i64,
        reply: CommentReply,
    ) -> BoxFuture<'_, Result<AdminComment, ModerationError>>;

    /// Removes comment `id` and every reply to it.
    ///
    /// # Errors
    ///
    /// [`ModerationError::NotFound`] when there is no such comment, and
    /// [`ModerationError::Store`] when it cannot be written.
    fn delete(&self, id: i64) -> BoxFuture<'_, Result<(), ModerationError>>;
}

/// Why a comment could not be moderated.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ModerationError {
    /// No comment has that identity.
    #[error("no comment has that identity")]
    NotFound,
    /// A field of a reply cannot be kept, such as an empty text.
    #[error("the reply's {field} cannot be kept: {reason}")]
    Invalid {
        /// The field, such as `text`.
        field: &'static str,
        /// What is wrong with it, in plain words.
        reason: String,
    },
    /// The comments could not be read or written.
    #[error("the comments could not be read or written")]
    Store(#[source] Box<dyn Error + Send + Sync>),
}

/// The comments of a site that keeps none: the list is empty, and every
/// comment is not found.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoComments;

impl CommentModeration for NoComments {
    fn list(
        &self,
        _status: Option<CommentStatus>,
        _limit: u32,
    ) -> BoxFuture<'_, Result<Vec<AdminComment>, ModerationError>> {
        Box::pin(async { Ok(Vec::new()) })
    }

    fn approve(&self, _id: i64) -> BoxFuture<'_, Result<AdminComment, ModerationError>> {
        Box::pin(async { Err(ModerationError::NotFound) })
    }

    fn reply(
        &self,
        _id: i64,
        _reply: CommentReply,
    ) -> BoxFuture<'_, Result<AdminComment, ModerationError>> {
        Box::pin(async { Err(ModerationError::NotFound) })
    }

    fn delete(&self, _id: i64) -> BoxFuture<'_, Result<(), ModerationError>> {
        Box::pin(async { Err(ModerationError::NotFound) })
    }
}
