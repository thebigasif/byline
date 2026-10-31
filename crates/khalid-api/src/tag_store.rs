//! Where the Admin API reads and changes the site's tags. A tag lives in
//! the `tags` of its posts, so a store reads posts and rewrites their tags;
//! the grouping and renaming every store shares live here.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use khalid_core::{Locale, Slug, SlugMode};

use crate::content_store::BoxFuture;
use crate::entry_store::{EntryError, EntryStoreError};
use crate::resources::{AdminTag, TagEdit};

/// The site's tags, for the Admin API. `khalid serve` reads and rewrites
/// the post files; `MemoryTags`, from the `test-util` feature, keeps posts
/// in memory for tests.
pub trait TagStore: Send + Sync {
    /// Every tag, ordered by its identity.
    ///
    /// # Errors
    ///
    /// Returns [`EntryStoreError`] when the posts cannot be read.
    fn list(&self) -> BoxFuture<'_, Result<Vec<AdminTag>, EntryStoreError>>;

    /// Renames tag `id` in every post that has it, when none of them
    /// changed since the copy the rename was made on (BL-API-7).
    ///
    /// # Errors
    ///
    /// Returns [`EntryError::NotFound`], [`EntryError::Stale`],
    /// [`EntryError::Unsupported`] for a name no address can be made
    /// from, and [`EntryError::Store`].
    fn rename<'a>(
        &'a self,
        id: &'a str,
        edit: TagEdit,
    ) -> BoxFuture<'a, Result<AdminTag, EntryError>>;

    /// Removes tag `id` from every post that has it, when none of them
    /// changed since `updated_at`.
    ///
    /// # Errors
    ///
    /// Returns [`EntryError::NotFound`], [`EntryError::Stale`] and
    /// [`EntryError::Store`].
    fn remove<'a>(
        &'a self,
        id: &'a str,
        updated_at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<(), EntryError>>;
}

/// One post's tags, as a store reads them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaggedPost {
    /// The post's language.
    pub language: Locale,
    /// Its tags' names, in the order written.
    pub tags: Vec<String>,
    /// When the post last changed.
    pub updated_at: DateTime<Utc>,
}

/// The identity of the tag `name` in `language`: `<language>.<slug>`, or
/// `None` when no slug can be made from the name.
pub fn tag_id(language: &Locale, name: &str, slugs: SlugMode) -> Option<String> {
    let slug = Slug::from_title(name, language, slugs).ok()?;
    Some(format!("{language}.{slug}"))
}

/// Every tag `posts` carry, ordered by identity, each named as the first
/// post that has it writes it.
pub fn tags_of(posts: &[TaggedPost], slugs: SlugMode) -> Vec<AdminTag> {
    let mut tags: BTreeMap<String, (Locale, Slug, &str, usize, DateTime<Utc>)> = BTreeMap::new();
    for post in posts {
        for name in &post.tags {
            let Ok(slug) = Slug::from_title(name, &post.language, slugs) else {
                continue;
            };
            let id = format!("{}.{slug}", post.language);
            let tag = tags
                .entry(id)
                .or_insert_with(|| (post.language.clone(), slug, name, 0, post.updated_at));
            tag.3 += 1;
            tag.4 = tag.4.max(post.updated_at);
        }
    }
    let made = |(language, slug, name, posts, at): (Locale, Slug, &str, usize, _)| {
        AdminTag::new((language, slug), name, posts, at)
    };
    tags.into_values().map(made).collect()
}

/// The tags `post` has once tag `id` is renamed to `name`, or removed when
/// `name` is `None`: `None` when the post does not have the tag. A rename
/// onto a tag the post already has keeps one of the two.
pub fn retagged(
    post: &TaggedPost,
    id: &str,
    name: Option<&str>,
    slugs: SlugMode,
) -> Option<Vec<String>> {
    let id_of = |tag: &str| tag_id(&post.language, tag, slugs);
    if !post
        .tags
        .iter()
        .any(|tag| id_of(tag).as_deref() == Some(id))
    {
        return None;
    }
    let mut kept: Vec<String> = Vec::with_capacity(post.tags.len());
    let mut seen: Vec<Option<String>> = Vec::with_capacity(post.tags.len());
    for tag in &post.tags {
        let tag = match (id_of(tag).as_deref() == Some(id), name) {
            (true, Some(name)) => name,
            (true, None) => continue,
            (false, _) => tag.as_str(),
        };
        let tag_id = id_of(tag);
        if !seen.contains(&tag_id) {
            seen.push(tag_id);
            kept.push(tag.to_owned());
        }
    }
    Some(kept)
}

/// The error for a tag name no address can be made from.
pub fn unnamed() -> EntryError {
    EntryError::Unsupported {
        field: "name",
        why: "A tag's address is made from its name, which needs a letter or a digit.",
    }
}

/// The tag `id` among `posts`, when none of its posts changed since `seen`.
///
/// # Errors
///
/// Returns [`EntryError::NotFound`] and [`EntryError::Stale`].
pub fn current(
    posts: &[TaggedPost],
    id: &str,
    seen: DateTime<Utc>,
    slugs: SlugMode,
) -> Result<AdminTag, EntryError> {
    let tag = tags_of(posts, slugs).into_iter().find(|tag| tag.id() == id);
    let tag = tag.ok_or(EntryError::NotFound)?;
    if tag.updated_at() == seen {
        Ok(tag)
    } else {
        Err(EntryError::Stale {
            updated_at: tag.updated_at(),
        })
    }
}
