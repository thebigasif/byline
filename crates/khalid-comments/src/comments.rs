//! Sending, moderating and showing comments: the one door every caller uses.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use khalid_core::Clock;

use crate::{
    AuthorName, Comment, CommentAuthor, CommentId, CommentStatus, CommentStore, CommentStoreError,
    CommentText, NewComment, PostFile, Thread, threads,
};

/// The comments of one site, over a [`CommentStore`].
///
/// The reader form and the Admin API both call these methods, so a rule
/// such as "a reader's comment waits for approval" holds whichever way a
/// comment arrives (ADR 0003).
#[derive(Debug)]
pub struct Comments<S> {
    store: S,
    clock: Box<dyn Clock>,
    events: Vec<Arc<dyn CommentEvents>>,
}

/// What a running site does when comments change, such as sending the
/// `comment.added` webhook or rendering a post page again. Each method
/// returns at once; slow work belongs on a task of its own.
pub trait CommentEvents: fmt::Debug + Send + Sync {
    /// A comment was stored, a reader's or a staff reply.
    fn added(&self, comment: &Comment) {
        let _ = comment;
    }

    /// The approved comments on `post` changed, so its page shows others.
    fn shown_changed(&self, post: &PostFile) {
        let _ = post;
    }
}

/// Why a comment could not be sent or moderated.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CommentsError {
    /// No comment has this identity.
    #[error("there is no comment {id}")]
    NotFound {
        /// The identity asked for.
        id: CommentId,
    },
    /// A reader answered a comment that is not shown on the same post.
    #[error("comment {id} cannot be answered on this post")]
    ParentNotFound {
        /// The identity answered.
        id: CommentId,
    },
    /// The store failed.
    #[error(transparent)]
    Store(#[from] CommentStoreError),
}

impl<S: CommentStore> Comments<S> {
    /// The comments in `store`, timed by `clock`.
    pub fn new(store: S, clock: impl Clock + 'static) -> Self {
        Self {
            store,
            clock: Box::new(clock),
            events: Vec::new(),
        }
    }

    /// These comments, telling `events` about each change after the
    /// listeners added before.
    #[must_use]
    pub fn with_events(mut self, events: Arc<dyn CommentEvents>) -> Self {
        self.events.push(events);
        self
    }

    /// Stores a reader's comment to wait for approval. A reply must answer
    /// an approved comment on the same post; a reply to a reply is stored
    /// under the comment that started the thread.
    ///
    /// # Errors
    ///
    /// [`CommentsError::ParentNotFound`] when the comment answered is not
    /// shown on that post, and [`CommentsError::Store`] when the store fails.
    pub async fn submit(&self, mut new: NewComment) -> Result<Comment, CommentsError> {
        if let Some(id) = new.parent {
            let parent = self.store.get(id).await?;
            let shown = parent.filter(|parent| {
                parent.post == new.post && parent.status == CommentStatus::Approved
            });
            let parent = shown.ok_or(CommentsError::ParentNotFound { id })?;
            new.parent = Some(parent.parent.unwrap_or(parent.id));
        }
        let at = self.clock.now();
        let sent = self.store.insert(new, CommentStatus::Pending, at).await?;
        self.events.iter().for_each(|events| events.added(&sent));
        Ok(sent)
    }

    /// Shows comment `id` under its post, and returns it.
    ///
    /// # Errors
    ///
    /// [`CommentsError::NotFound`] when there is no such comment, and
    /// [`CommentsError::Store`] when the store fails.
    pub async fn approve(&self, id: CommentId) -> Result<Comment, CommentsError> {
        let approved = self.approved(id).await?;
        self.shown_changed(&approved.post);
        Ok(approved)
    }

    async fn approved(&self, id: CommentId) -> Result<Comment, CommentsError> {
        let approved = self.store.set_status(id, CommentStatus::Approved).await?;
        approved.ok_or(CommentsError::NotFound { id })
    }

    fn shown_changed(&self, post: &PostFile) {
        for events in &self.events {
            events.shown_changed(post);
        }
    }

    /// Answers comment `id` as `name`, on the staff, and returns the
    /// reply. The reply is shown at once, and so is the comment it
    /// answers: replying to a comment approves it.
    ///
    /// # Errors
    ///
    /// [`CommentsError::NotFound`] when there is no such comment, and
    /// [`CommentsError::Store`] when the store fails.
    pub async fn reply(
        &self,
        id: CommentId,
        name: AuthorName,
        text: CommentText,
    ) -> Result<Comment, CommentsError> {
        let parent = self.approved(id).await?;
        let new = NewComment {
            post: parent.post,
            parent: Some(parent.parent.unwrap_or(parent.id)),
            author: CommentAuthor::Staff { name },
            text,
        };
        let at = self.clock.now();
        let reply = self.store.insert(new, CommentStatus::Approved, at).await?;
        self.events.iter().for_each(|events| events.added(&reply));
        self.shown_changed(&reply.post);
        Ok(reply)
    }

    /// Removes comment `id` and its replies, and returns it as it was.
    ///
    /// # Errors
    ///
    /// [`CommentsError::NotFound`] when there is no such comment, and
    /// [`CommentsError::Store`] when the store fails.
    pub async fn delete(&self, id: CommentId) -> Result<Comment, CommentsError> {
        let deleted = self.store.delete(id).await?;
        let deleted = deleted.ok_or(CommentsError::NotFound { id })?;
        if deleted.status == CommentStatus::Approved {
            self.shown_changed(&deleted.post);
        }
        Ok(deleted)
    }

    /// The newest `limit` comments with `status`, or with any status,
    /// newest first: the moderation queue.
    ///
    /// # Errors
    ///
    /// [`CommentsError::Store`] when the store fails.
    pub async fn listed(
        &self,
        status: Option<CommentStatus>,
        limit: u32,
    ) -> Result<Vec<Comment>, CommentsError> {
        Ok(self.store.newest(status, limit).await?)
    }

    /// The approved comments on `post`, as its page shows them.
    ///
    /// # Errors
    ///
    /// [`CommentsError::Store`] when the store fails.
    pub async fn shown_on(&self, post: &PostFile) -> Result<Vec<Thread>, CommentsError> {
        let approved = self
            .store
            .with_status(CommentStatus::Approved, Some(post))
            .await?;
        Ok(threads(approved))
    }

    /// The approved comments on every post, as their pages show them.
    ///
    /// # Errors
    ///
    /// [`CommentsError::Store`] when the store fails.
    pub async fn shown(&self) -> Result<BTreeMap<PostFile, Vec<Thread>>, CommentsError> {
        let approved = self
            .store
            .with_status(CommentStatus::Approved, None)
            .await?;
        let mut by_post: BTreeMap<PostFile, Vec<Comment>> = BTreeMap::new();
        for comment in approved {
            by_post
                .entry(comment.post.clone())
                .or_default()
                .push(comment);
        }
        Ok(by_post
            .into_iter()
            .map(|(post, comments)| (post, threads(comments)))
            .collect())
    }
}
