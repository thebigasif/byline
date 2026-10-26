//! Rebuilding the content index from the files (ADR 0001, ADR 0010).

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
    /// files in one language that share a translation key, or two files with
    /// the same path.
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
        tx.commit().await?;
        Ok(RebuildSummary {
            posts: index.posts.len(),
        })
    }
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
