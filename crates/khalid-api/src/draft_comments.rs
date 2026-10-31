//! Comments staff leave on a draft for each other: threaded, attached to
//! a block of the body, resolved when dealt with (KH-EDT-4, plan section
//! 4.6). Readers never see them.

use std::error::Error;
use std::fmt;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::content_store::BoxFuture;
use crate::resources::AdminPost;

/// The most characters one comment may hold.
pub const MAX_DRAFT_COMMENT: usize = 5_000;

/// What a comment says: 1 to 5,000 characters, not only spaces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(try_from = "String")]
pub struct DraftCommentText(String);

impl DraftCommentText {
    /// The text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for DraftCommentText {
    type Error = &'static str;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        if text.trim().is_empty() {
            Err("text must say something")
        } else if text.chars().count() > MAX_DRAFT_COMMENT {
            Err("text must be at most 5000 characters")
        } else {
            Ok(Self(text))
        }
    }
}

/// One comment on a draft.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
#[non_exhaustive]
pub struct DraftComment {
    /// The identity the store gave it.
    pub id: i64,
    /// The comment it answers; empty for the first comment of a thread.
    pub parent: Option<i64>,
    /// The block of the body it is about, counted from 1, or empty for the
    /// post as a whole. A reply is about its thread's block.
    pub block: Option<u32>,
    /// The key or staff account that wrote it.
    pub author: String,
    /// What it says.
    pub text: DraftCommentText,
    /// Whether its thread has been dealt with. Only a thread's first
    /// comment carries it.
    pub resolved: bool,
    /// When it was written.
    pub created_at: DateTime<Utc>,
}

impl DraftComment {
    /// `new` as a store saved it, as `id` at `created_at`, with its thread
    /// open.
    pub fn saved(id: i64, new: NewDraftComment, created_at: DateTime<Utc>) -> Self {
        Self {
            id,
            parent: new.parent,
            block: new.block,
            author: new.author,
            text: new.text,
            resolved: false,
            created_at,
        }
    }

    /// The same comment, with its thread in `state`.
    #[must_use]
    pub fn in_state(self, state: ThreadState) -> Self {
        let resolved = state.is_resolved();
        Self { resolved, ..self }
    }
}

/// A comment to add to post `post`, checked by the route.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct NewDraftComment {
    /// The post's identity.
    pub post: String,
    /// The first comment of the thread it answers, if any.
    pub parent: Option<i64>,
    /// The block it is about, counted from 1.
    pub block: Option<u32>,
    /// Who wrote it.
    pub author: String,
    /// What it says.
    pub text: DraftCommentText,
}

impl NewDraftComment {
    /// A comment by `author` on `post`, about the post as a whole.
    pub fn new(post: &str, author: &str, text: DraftCommentText) -> Self {
        Self {
            post: post.to_owned(),
            parent: None,
            block: None,
            author: author.to_owned(),
            text,
        }
    }
}

/// The comments on drafts. `khalid-db` keeps them in SQLite as visitor
/// data, which a rebuild never touches; `MemoryDraftComments`, from the
/// `test-util` feature, keeps them in memory for tests.
pub trait DraftCommentStore: Send + Sync {
    /// Every comment on post `post`, oldest first.
    ///
    /// # Errors
    ///
    /// [`DraftCommentStoreError`] when they cannot be read.
    fn list<'a>(
        &'a self,
        post: &'a str,
    ) -> BoxFuture<'a, Result<Vec<DraftComment>, DraftCommentStoreError>>;

    /// Saves `comment`, dated now. The caller has checked its parent.
    ///
    /// # Errors
    ///
    /// [`DraftCommentStoreError`] when it cannot be written.
    fn add(
        &self,
        comment: NewDraftComment,
    ) -> BoxFuture<'_, Result<DraftComment, DraftCommentStoreError>>;

    /// Puts the thread that comment `id` on post `post` begins in `state`,
    /// answering whether there was one.
    ///
    /// # Errors
    ///
    /// [`DraftCommentStoreError`] when it cannot be written.
    fn resolve<'a>(
        &'a self,
        post: &'a str,
        id: i64,
        state: ThreadState,
    ) -> BoxFuture<'a, Result<bool, DraftCommentStoreError>>;
}

/// Whether a thread still needs someone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    /// Waiting for an answer or a change.
    Open,
    /// Dealt with.
    Resolved,
}

/// `Resolved` for `true`, as the API sends it.
impl From<bool> for ThreadState {
    fn from(resolved: bool) -> Self {
        if resolved { Self::Resolved } else { Self::Open }
    }
}

impl ThreadState {
    /// Whether the thread is dealt with.
    pub fn is_resolved(self) -> bool {
        self == Self::Resolved
    }
}

/// The comments on drafts could not be read or written.
#[derive(Debug, thiserror::Error)]
#[error("the comments on drafts could not be read or written")]
pub struct DraftCommentStoreError(#[source] Box<dyn Error + Send + Sync>);

impl DraftCommentStoreError {
    /// Wraps what the storage reported.
    pub fn storage(source: impl Error + Send + Sync + 'static) -> Self {
        Self(Box::new(source))
    }
}

/// Who hears about a new comment: the binary emails the people in the
/// thread and the site's editors.
pub trait DraftCommentNotice: Send + Sync {
    /// `comment` was added to `post`, whose comments before it were
    /// `earlier`, oldest first. Called after it is saved; a failure to
    /// tell anyone is the receiver's to log, never the writer's to see.
    fn added<'a>(
        &'a self,
        post: &'a AdminPost,
        comment: &'a DraftComment,
        earlier: &'a [DraftComment],
    ) -> BoxFuture<'a, ()>;
}

/// Tells nobody.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoNotice;

impl DraftCommentNotice for NoNotice {
    fn added<'a>(
        &'a self,
        _post: &'a AdminPost,
        _comment: &'a DraftComment,
        _earlier: &'a [DraftComment],
    ) -> BoxFuture<'a, ()> {
        Box::pin(async {})
    }
}

/// The comments on drafts and who hears about new ones, as the Admin API
/// holds them. Cloning shares both.
#[derive(Clone)]
pub struct DraftComments {
    store: Arc<dyn DraftCommentStore>,
    notice: Arc<dyn DraftCommentNotice>,
}

impl fmt::Debug for DraftComments {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DraftComments").finish_non_exhaustive()
    }
}

impl DraftComments {
    /// The comments in `store`, telling nobody about new ones.
    pub fn new(store: Arc<dyn DraftCommentStore>) -> Self {
        let notice = Arc::new(NoNotice);
        Self { store, notice }
    }

    /// The same comments, telling `notice` about each new one.
    #[must_use]
    pub fn with_notice(self, notice: Arc<dyn DraftCommentNotice>) -> Self {
        Self { notice, ..self }
    }

    /// Where the comments are kept.
    pub fn store(&self) -> &dyn DraftCommentStore {
        self.store.as_ref()
    }

    /// Who hears about new comments.
    pub fn notice(&self) -> &dyn DraftCommentNotice {
        self.notice.as_ref()
    }
}

#[cfg(any(test, feature = "test-util"))]
pub use memory::MemoryDraftComments;

#[cfg(any(test, feature = "test-util"))]
mod memory {
    use std::fmt;
    use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

    use khalid_core::{Clock, SystemClock};

    use super::{
        BoxFuture, DraftComment, DraftCommentStore, DraftCommentStoreError, NewDraftComment,
        ThreadState,
    };

    /// Comments on drafts held in memory, for tests.
    pub struct MemoryDraftComments {
        rows: Mutex<Vec<(String, DraftComment)>>,
        clock: Arc<dyn Clock>,
    }

    impl fmt::Debug for MemoryDraftComments {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("MemoryDraftComments")
                .finish_non_exhaustive()
        }
    }

    impl Default for MemoryDraftComments {
        fn default() -> Self {
            Self::with_clock(SystemClock)
        }
    }

    impl MemoryDraftComments {
        /// No comments, dated by `clock`.
        pub fn with_clock(clock: impl Clock + 'static) -> Self {
            let rows = Mutex::default();
            let clock = Arc::new(clock);
            Self { rows, clock }
        }

        fn rows(&self) -> MutexGuard<'_, Vec<(String, DraftComment)>> {
            self.rows.lock().unwrap_or_else(PoisonError::into_inner)
        }
    }

    impl DraftCommentStore for MemoryDraftComments {
        fn list<'a>(
            &'a self,
            post: &'a str,
        ) -> BoxFuture<'a, Result<Vec<DraftComment>, DraftCommentStoreError>> {
            let rows = self.rows();
            let found: Vec<DraftComment> = rows
                .iter()
                .filter(|(on, _)| on == post)
                .map(|(_, comment)| comment.clone())
                .collect();
            Box::pin(async move { Ok(found) })
        }

        fn add(
            &self,
            new: NewDraftComment,
        ) -> BoxFuture<'_, Result<DraftComment, DraftCommentStoreError>> {
            let mut rows = self.rows();
            let id = i64::try_from(rows.len())
                .unwrap_or(i64::MAX)
                .saturating_add(1);
            let post = new.post.clone();
            let comment = DraftComment::saved(id, new, self.clock.now());
            rows.push((post, comment.clone()));
            Box::pin(async move { Ok(comment) })
        }

        fn resolve<'a>(
            &'a self,
            post: &'a str,
            id: i64,
            state: ThreadState,
        ) -> BoxFuture<'a, Result<bool, DraftCommentStoreError>> {
            let mut rows = self.rows();
            let first = rows
                .iter_mut()
                .find(|(on, comment)| on == post && comment.id == id && comment.parent.is_none());
            let found = first
                .map(|(_, comment)| comment.resolved = state.is_resolved())
                .is_some();
            Box::pin(async move { Ok(found) })
        }
    }
}
