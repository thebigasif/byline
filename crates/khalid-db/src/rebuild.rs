//! Rebuilding the content index from the files (ADR 0001, ADR 0010).

use std::collections::BTreeSet;

use khalid_core::Slug;
use sqlx::{Sqlite, Transaction};

use crate::{ContentIndex, Database, DbError, IndexedPost, SiteId};

/// What a rebuild wrote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct RebuildSummary {
    posts: usize,
}

impl RebuildSummary {
    /// How many posts are in the index now.
    pub fn posts(&self) -> usize {
        self.posts
    }
}

impl Database {
    /// The site named `handle`, added to `sites` the first time it is asked
    /// for. The same handle always gives the same [`SiteId`].
    ///
    /// # Errors
    ///
    /// Returns [`DbError::Query`] when the database cannot be written.
    pub async fn site(&self, handle: &Slug) -> Result<SiteId, DbError> {
        let handle = handle.as_str();
        let id = sqlx::query_scalar!(
            "INSERT INTO sites (handle) VALUES (?1) \
             ON CONFLICT (handle) DO UPDATE SET handle = excluded.handle \
             RETURNING id",
            handle
        )
        .fetch_one(&self.writer)
        .await?;
        Ok(SiteId::from_row(id))
    }

    /// Replaces the site's derived rows with `index`, in one transaction.
    /// Visitor tables and other sites are not touched. Readers see the old
    /// index until the new one is complete, and a failed rebuild leaves the
    /// old index in place.
    ///
    /// # Errors
    ///
    /// Returns [`DbError::Query`] when a row cannot be written, such as two
    /// files in one language that share a translation key, two files with
    /// the same path, or two authors, categories or tags with one slug in
    /// one language.
    pub async fn rebuild(
        &self,
        site: SiteId,
        index: &ContentIndex,
    ) -> Result<RebuildSummary, DbError> {
        let mut tx = self.writer.begin().await?;
        clear(&mut tx, site).await?;
        for post in &index.posts {
            insert_post(&mut tx, site, post).await?;
        }
        insert_lists(&mut tx, site, index).await?;
        tx.commit().await?;
        Ok(RebuildSummary {
            posts: index.posts.len(),
        })
    }

    /// Brings the site's index up to `index` after `changes`, in one
    /// transaction: the posts named in `changes` are written again or
    /// removed, with their translation keys, and the authors, categories
    /// and tags are written again whole. Every other post row is left as it
    /// is, so a save costs the same on a site of ten posts or ten thousand.
    /// A failed update leaves the index as it was.
    ///
    /// # Errors
    ///
    /// Returns [`DbError::Query`] when a row cannot be written, for the
    /// same reasons as [`Database::rebuild`].
    pub async fn update(
        &self,
        site: SiteId,
        index: &ContentIndex,
        changes: &PostChanges,
    ) -> Result<RebuildSummary, DbError> {
        let mut tx = self.writer.begin().await?;
        let row = site.as_row();
        for path in changes.written.iter().chain(&changes.removed) {
            sqlx::query!(
                "DELETE FROM translations WHERE site_id = ?1 AND path = ?2",
                row,
                path
            )
            .execute(&mut *tx)
            .await?;
            sqlx::query!(
                "DELETE FROM posts WHERE site_id = ?1 AND path = ?2",
                row,
                path
            )
            .execute(&mut *tx)
            .await?;
        }
        let written = index
            .posts
            .iter()
            .filter(|post| changes.written.contains(&post.path));
        for post in written {
            insert_post(&mut tx, site, post).await?;
        }
        sqlx::query!("DELETE FROM tags WHERE site_id = ?1", row)
            .execute(&mut *tx)
            .await?;
        sqlx::query!("DELETE FROM categories WHERE site_id = ?1", row)
            .execute(&mut *tx)
            .await?;
        sqlx::query!("DELETE FROM authors WHERE site_id = ?1", row)
            .execute(&mut *tx)
            .await?;
        insert_lists(&mut tx, site, index).await?;
        tx.commit().await?;
        Ok(RebuildSummary {
            posts: index.posts.len(),
        })
    }
}

/// The posts that changed since the index was last written, by the path
/// each file has in the index.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct PostChanges {
    /// Files that are new or were edited, written again from the index.
    pub written: BTreeSet<String>,
    /// Files that are gone, whose rows are removed.
    pub removed: BTreeSet<String>,
}

/// Writes one post and, when it has one, its translation key.
async fn insert_post(
    tx: &mut Transaction<'_, Sqlite>,
    site: SiteId,
    post: &IndexedPost,
) -> Result<(), DbError> {
    let site = site.as_row();
    let locale = post.locale.as_str();
    let slug = post.slug.as_str();
    let author = post.author.as_ref().map(Slug::as_str);
    let category = post.category.as_ref().map(Slug::as_str);
    let hash = post.content_hash.as_slice();
    sqlx::query!(
        "INSERT INTO posts (site_id, locale, collection, slug, path, title, description, \
         date, author, category, content_hash) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        site,
        locale,
        post.collection,
        slug,
        post.path,
        post.title,
        post.description,
        post.date,
        author,
        category,
        hash,
    )
    .execute(&mut **tx)
    .await?;
    if let Some(key) = &post.translation {
        sqlx::query!(
            "INSERT INTO translations (site_id, key, locale, path) VALUES (?1, ?2, ?3, ?4)",
            site,
            key,
            locale,
            post.path,
        )
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

/// Writes the authors, categories and tags, each list in its order, which
/// the rows' identities keep.
async fn insert_lists(
    tx: &mut Transaction<'_, Sqlite>,
    site: SiteId,
    index: &ContentIndex,
) -> Result<(), DbError> {
    let site = site.as_row();
    for author in &index.authors {
        let (locale, slug) = (author.locale.as_str(), author.slug.as_str());
        let hash = author.content_hash.as_slice();
        sqlx::query!(
            "INSERT INTO authors (site_id, locale, slug, name, path, content_hash) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            site,
            locale,
            slug,
            author.name,
            author.path,
            hash,
        )
        .execute(&mut **tx)
        .await?;
    }
    for category in &index.categories {
        let (locale, slug) = (category.locale.as_str(), category.slug.as_str());
        sqlx::query!(
            "INSERT INTO categories (site_id, locale, slug, name) VALUES (?1, ?2, ?3, ?4)",
            site,
            locale,
            slug,
            category.name,
        )
        .execute(&mut **tx)
        .await?;
    }
    for tag in &index.tags {
        let (locale, slug) = (tag.locale.as_str(), tag.slug.as_str());
        sqlx::query!(
            "INSERT INTO tags (site_id, locale, slug, name) VALUES (?1, ?2, ?3, ?4)",
            site,
            locale,
            slug,
            tag.name,
        )
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

/// Runs `DELETE FROM <table> WHERE site_id = ?1` for each table, each one a
/// checked query with the table name fixed when the crate compiles.
macro_rules! delete_site_rows {
    ($tx:expr, $site:expr, $($table:literal),+ $(,)?) => {
        $(
            sqlx::query!("DELETE FROM " + $table + " WHERE site_id = ?1", $site)
                .execute(&mut **$tx)
                .await?;
        )+
    };
}

/// Deletes every derived row of `site`. The README lists the derived tables,
/// and a test fails if one is left out here. Deleting posts and tags also
/// deletes their `post_tags` rows by cascade.
async fn clear(tx: &mut Transaction<'_, Sqlite>, site: SiteId) -> Result<(), DbError> {
    let site = site.as_row();
    delete_site_rows!(
        tx,
        site,
        "translations",
        "links",
        "lint_results",
        "alias_redirects",
        "posts",
        "pages",
        "collection_entries",
        "tags",
        "categories",
        "authors",
    );
    Ok(())
}
