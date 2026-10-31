//! Tagged posts held in memory, for tests.

use std::sync::{Mutex, MutexGuard, PoisonError};

use chrono::{DateTime, Utc};
use khalid_core::{Clock, SlugMode, SystemClock};

use super::{TagStore, TaggedPost, current, retagged, tag_id, tags_of, unnamed};
use crate::content_store::BoxFuture;
use crate::entry_store::{EntryError, EntryStoreError};
use crate::resources::{AdminTag, TagEdit};

/// Posts and their tags held in memory, for tests, with ASCII slugs as on
/// a site with the default settings.
#[derive(Debug)]
pub struct MemoryTags {
    posts: Mutex<Vec<TaggedPost>>,
    clock: Box<dyn Clock>,
}

impl MemoryTags {
    /// A store holding `posts`, stamping changes with the system clock.
    pub fn new(posts: Vec<TaggedPost>) -> Self {
        Self {
            posts: Mutex::new(posts),
            clock: Box::new(SystemClock),
        }
    }

    /// The same store, stamping changes with `clock`.
    #[must_use]
    pub fn with_clock(self, clock: impl Clock + 'static) -> Self {
        Self {
            clock: Box::new(clock),
            ..self
        }
    }

    fn posts(&self) -> MutexGuard<'_, Vec<TaggedPost>> {
        self.posts.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Renames tag `id` to `name`, or removes it when `name` is `None`, in
    /// every post that has it, and returns the tag it became, if any.
    fn retag(
        &self,
        id: &str,
        name: Option<&str>,
        seen: DateTime<Utc>,
    ) -> Result<Option<AdminTag>, EntryError> {
        let mut posts = self.posts();
        let tag = current(&posts, id, seen, SlugMode::Ascii)?;
        let new_id = match name {
            Some(name) => Some(tag_id(tag.language(), name, SlugMode::Ascii).ok_or_else(unnamed)?),
            None => None,
        };
        let now = self.clock.now();
        for post in posts.iter_mut() {
            if let Some(tags) = retagged(post, id, name, SlugMode::Ascii) {
                post.tags = tags;
                post.updated_at = now;
            }
        }
        let tags = tags_of(&posts, SlugMode::Ascii);
        Ok(tags
            .into_iter()
            .find(|tag| Some(tag.id()) == new_id.as_deref()))
    }
}

/// Memory answers at once, so each future is ready when returned.
impl TagStore for MemoryTags {
    fn list(&self) -> BoxFuture<'_, Result<Vec<AdminTag>, EntryStoreError>> {
        let tags = tags_of(&self.posts(), SlugMode::Ascii);
        Box::pin(std::future::ready(Ok(tags)))
    }

    fn rename<'a>(
        &'a self,
        id: &'a str,
        edit: TagEdit,
    ) -> BoxFuture<'a, Result<AdminTag, EntryError>> {
        let renamed = self.retag(id, Some(edit.name()), edit.updated_at());
        let renamed = renamed.and_then(|tag| tag.ok_or(EntryError::NotFound));
        Box::pin(std::future::ready(renamed))
    }

    fn remove<'a>(
        &'a self,
        id: &'a str,
        updated_at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<(), EntryError>> {
        let removed = self.retag(id, None, updated_at).map(|_| ());
        Box::pin(std::future::ready(removed))
    }
}
