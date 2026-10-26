//! Where the Admin API reads and writes posts.

use std::error::Error;

use chrono::{DateTime, Utc};
use khalid_core::Slug;
use khalid_lint::Report;

use crate::content_store::BoxFuture;
use crate::resources::{AdminPost, PostDraft, PostEdit};

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
    /// check and the write happen together. When the edit publishes the
    /// post, the store runs the lint publish gate (`Linter::gate`) on the
    /// post as it would go live, and saves nothing when the gate refuses.
    ///
    /// # Errors
    ///
    /// Returns [`UpdatePostError::Stale`] naming the last change when the
    /// post changed after that copy, [`UpdatePostError::NotFound`] when no
    /// post has that identity, [`UpdatePostError::Refused`] with the lint
    /// report when the gate refuses, and [`UpdatePostError::Store`] when the
    /// posts cannot be read or written.
    fn update<'a>(
        &'a self,
        id: &'a str,
        edit: PostEdit,
        by: &'a str,
    ) -> BoxFuture<'a, Result<AdminPost, UpdatePostError>>;

    /// Saves `draft`, made by `by`, as a new draft post, with a slug made
    /// from its title when it has none. A second call with the same
    /// `idempotency` key returns the post the first call made, so a retried
    /// request never makes two posts (BL-API-7).
    ///
    /// # Errors
    ///
    /// Returns [`CreatePostError::SlugTaken`] when another post has the
    /// slug, [`CreatePostError::NoSlug`] when the title has nothing to make
    /// one from, and [`CreatePostError::Store`] when the posts cannot be
    /// read or written.
    fn create<'a>(
        &'a self,
        draft: PostDraft,
        by: &'a str,
        idempotency: Option<&'a str>,
    ) -> BoxFuture<'a, Result<AdminPost, CreatePostError>>;
}

/// Why a new post was not saved.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CreatePostError {
    /// Another post has the slug.
    #[error("another post has the slug {0}")]
    SlugTaken(Slug),
    /// The title has no letters or digits to make a slug from.
    #[error("the title has nothing to make a slug from")]
    NoSlug,
    /// The posts could not be read or written.
    #[error(transparent)]
    Store(#[from] PostStoreError),
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
    /// The edit publishes the post, and the lint publish gate refused it.
    /// The post stays as it was.
    #[error("the linter refused to publish the post")]
    Refused(Report),
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
    use std::collections::BTreeMap;
    use std::sync::{Mutex, MutexGuard, PoisonError};

    use khalid_core::{Clock, Locale, Slug, SlugMode, SystemClock};

    use super::{
        AdminPost, BoxFuture, CreatePostError, PostDraft, PostEdit, PostStore, PostStoreError,
        UpdatePostError,
    };

    /// Posts held in memory, for tests. It runs no publish gate: an edit
    /// that publishes always goes live.
    #[derive(Debug)]
    pub struct MemoryPostStore {
        posts: Mutex<Vec<AdminPost>>,
        seen: Mutex<BTreeMap<String, String>>,
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
                seen: Mutex::new(BTreeMap::new()),
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

        /// Adds a draft, making slugs in English ASCII as a site with the
        /// default settings would.
        fn add(
            &self,
            draft: &PostDraft,
            by: &str,
            idempotency: Option<&str>,
        ) -> Result<AdminPost, CreatePostError> {
            let mut seen = self.seen.lock().unwrap_or_else(PoisonError::into_inner);
            let mut posts = self.posts();
            let earlier = idempotency.and_then(|key| seen.get(key));
            if let Some(post) = earlier.and_then(|id| posts.iter().find(|post| post.id() == id)) {
                return Ok(post.clone());
            }
            let slug = if let Some(slug) = draft.slug() {
                slug.clone()
            } else {
                let english = Locale::try_new("en").map_err(|_| CreatePostError::NoSlug)?;
                Slug::from_title(draft.title(), &english, SlugMode::Ascii)
                    .map_err(|_| CreatePostError::NoSlug)?
            };
            if posts.iter().any(|post| post.slug() == &slug) {
                return Err(CreatePostError::SlugTaken(slug));
            }
            let at = self.clock.now();
            let post = AdminPost::new(
                slug.as_str(),
                slug.clone(),
                draft.title(),
                draft.markdown(),
                at,
            )
            .with_author(draft.author().cloned())
            .with_updated_by(by);
            if let Some(key) = idempotency {
                seen.insert(key.to_owned(), post.id().to_owned());
            }
            posts.push(post.clone());
            Ok(post)
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

        fn create<'a>(
            &'a self,
            draft: PostDraft,
            by: &'a str,
            idempotency: Option<&'a str>,
        ) -> BoxFuture<'a, Result<AdminPost, CreatePostError>> {
            Box::pin(std::future::ready(self.add(&draft, by, idempotency)))
        }
    }
}
