//! Where the Admin API reads and writes posts.

use std::error::Error;

use chrono::{DateTime, Utc};
use khalid_core::Slug;
use khalid_lint::Report;

use crate::content_store::{BoxFuture, Listed};
use crate::publish_door::ScheduleError;
use crate::query::ListQuery;
use crate::resources::{AdminPost, NotRealUpdate, PostDraft, PostEdit, PostRow};

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

    /// One page of the posts, in every status, as rows of the admin's
    /// posts table with their lint counts, filtered and ordered as `query`
    /// says. The caller has checked the query's fields against
    /// [`PostRow`]'s; [`select`](crate::content_store::select) does the
    /// rest for a store that reads every post.
    ///
    /// # Errors
    ///
    /// Returns [`PostStoreError`] when the posts or the lint settings
    /// cannot be read.
    fn list<'a>(
        &'a self,
        query: &'a ListQuery,
    ) -> BoxFuture<'a, Result<Listed<PostRow>, PostStoreError>>;

    /// Applies `edit`, made by `by`, to post `id`, but only when the post
    /// has not changed since the copy the edit was made on (BL-API-7). The
    /// check and the write happen together. When the edit publishes the
    /// post, the store runs the lint publish gate (`Linter::gate`) on the
    /// post as it would go live, and saves nothing when the gate refuses.
    ///
    /// # Errors
    ///
    /// Returns [`UpdatePostError::Stale`] naming the last change when the
    /// post changed after that copy, [`UpdatePostError::SlugTaken`] when
    /// the edit's new slug is another post's, [`UpdatePostError::Unsupported`]
    /// when the store cannot keep a change, [`UpdatePostError::NotFound`] when no
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

    /// Removes post `id`, in any status, but only when it has not changed
    /// since `updated_at`, the copy the caller saw (BL-API-7). The check and
    /// the removal happen together. Returns the post as it was. A store
    /// that is also a [`PostTrash`](crate::post_trash::PostTrash) keeps it
    /// there, so it can be restored.
    ///
    /// # Errors
    ///
    /// Returns [`DeletePostError::NotFound`] when no post has that identity,
    /// [`DeletePostError::Stale`] naming the last change when the post
    /// changed after that copy, and [`DeletePostError::Store`] when the posts
    /// cannot be read or written.
    fn delete<'a>(
        &'a self,
        id: &'a str,
        updated_at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<AdminPost, DeletePostError>>;
}

/// Why a post was not deleted.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DeletePostError {
    /// No post has that identity.
    #[error("no post has that identity")]
    NotFound,
    /// The post changed after the copy the caller saw.
    #[error("the post changed after the copy the delete was made on")]
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
    /// The edit gives the post a slug another post has.
    #[error("another post has the slug {0}")]
    SlugTaken(Slug),
    /// The store cannot keep one of the edit's changes, such as a setting
    /// its files have no field for yet. Nothing was saved.
    #[error("the store cannot keep {setting}: {why}")]
    Unsupported {
        /// The field or setting, as the edit names it, such as `co_authors`.
        setting: &'static str,
        /// Why, in one plain sentence for the person who sent it.
        why: &'static str,
    },
    /// The edit publishes the post, and the lint publish gate refused it.
    /// The post stays as it was.
    #[error("the linter refused to publish the post")]
    Refused(Report),
    /// The edit claims a real update that it is not. The post stays as it
    /// was.
    #[error("the edit is not a real update")]
    NotRealUpdate(#[source] NotRealUpdate),
    /// The edit's schedule cannot be kept. The post stays as it was.
    #[error("the schedule cannot be kept")]
    Schedule(#[source] ScheduleError),
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

    use chrono::{DateTime, Utc};

    use super::{
        AdminPost, BoxFuture, CreatePostError, DeletePostError, ListQuery, Listed, PostDraft,
        PostEdit, PostRow, PostStore, PostStoreError, UpdatePostError,
    };
    use crate::content_store::select;
    use crate::post_trash::{PostTrash, TrashError, Trashed};
    use crate::resources::LintCounts;

    /// Posts held in memory, for tests. It runs no publish gate: an edit
    /// that publishes always goes live, and a post's lint counts are
    /// whatever [`MemoryPostStore::with_lint`] gave it, or none. A deleted
    /// post waits in its trash.
    #[derive(Debug)]
    pub struct MemoryPostStore {
        posts: Mutex<Vec<AdminPost>>,
        trash: Mutex<Vec<(Trashed, AdminPost)>>,
        seen: Mutex<BTreeMap<String, String>>,
        lint: BTreeMap<String, LintCounts>,
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
                trash: Mutex::new(Vec::new()),
                seen: Mutex::new(BTreeMap::new()),
                lint: BTreeMap::new(),
                clock: Box::new(SystemClock),
            }
        }

        /// The same store, listing post `id` with `counts` lint findings.
        #[must_use]
        pub fn with_lint(mut self, id: &str, counts: LintCounts) -> Self {
            self.lint.insert(id.to_owned(), counts);
            self
        }

        fn rows(&self, query: &ListQuery) -> Listed<PostRow> {
            let rows: Vec<PostRow> = self
                .posts()
                .iter()
                .map(|post| {
                    let counts = self.lint.get(post.id()).copied().unwrap_or_default();
                    PostRow::new(post, counts.gate()).with_lint(counts)
                })
                .collect();
            select(rows.iter(), query)
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
            let taken = |slug: &Slug| posts.iter().any(|p| p.id() != id && p.slug() == slug);
            if let Some(slug) = edit.slug().filter(|slug| taken(slug)) {
                return Err(UpdatePostError::SlugTaken(slug.clone()));
            }
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

        fn remove(&self, id: &str, seen: DateTime<Utc>) -> Result<AdminPost, DeletePostError> {
            let mut posts = self.posts();
            let at = posts
                .iter()
                .position(|post| post.id() == id)
                .ok_or(DeletePostError::NotFound)?;
            let post = posts.remove(at);
            if post.updated_at() != seen {
                let stale = DeletePostError::Stale {
                    updated_at: post.updated_at(),
                    updated_by: post.updated_by().map(str::to_owned),
                };
                posts.insert(at, post);
                return Err(stale);
            }
            let trashed = Trashed::new(&post, self.clock.now());
            self.trash().push((trashed, post.clone()));
            Ok(post)
        }

        fn trash(&self) -> MutexGuard<'_, Vec<(Trashed, AdminPost)>> {
            self.trash.lock().unwrap_or_else(PoisonError::into_inner)
        }

        fn take_back(&self, key: &str) -> Result<AdminPost, TrashError> {
            let mut posts = self.posts();
            let mut trash = self.trash();
            let at = trash
                .iter()
                .position(|(trashed, _)| trashed.key == key)
                .ok_or(TrashError::NotFound)?;
            let (trashed, post) = trash.remove(at);
            if posts.iter().any(|kept| kept.slug() == post.slug()) {
                trash.insert(at, (trashed, post.clone()));
                let id = post.id().to_owned();
                return Err(TrashError::Taken { id });
            }
            posts.push(post.clone());
            Ok(post)
        }

        fn forget(&self, key: &str) -> Result<(), TrashError> {
            let mut trash = self.trash();
            let before = trash.len();
            trash.retain(|(trashed, _)| trashed.key != key);
            if trash.len() == before {
                return Err(TrashError::NotFound);
            }
            Ok(())
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

        fn list<'a>(
            &'a self,
            query: &'a ListQuery,
        ) -> BoxFuture<'a, Result<Listed<PostRow>, PostStoreError>> {
            Box::pin(std::future::ready(Ok(self.rows(query))))
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

        fn delete<'a>(
            &'a self,
            id: &'a str,
            updated_at: DateTime<Utc>,
        ) -> BoxFuture<'a, Result<AdminPost, DeletePostError>> {
            Box::pin(std::future::ready(self.remove(id, updated_at)))
        }
    }

    impl PostTrash for MemoryPostStore {
        fn trashed(&self) -> BoxFuture<'_, Result<Vec<Trashed>, PostStoreError>> {
            let trash = self.trash();
            let listed = trash.iter().rev().map(|(trashed, _)| trashed.clone());
            Box::pin(std::future::ready(Ok(listed.collect())))
        }

        fn restore<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<AdminPost, TrashError>> {
            Box::pin(std::future::ready(self.take_back(key)))
        }

        fn purge<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<(), TrashError>> {
            Box::pin(std::future::ready(self.forget(key)))
        }
    }
}
