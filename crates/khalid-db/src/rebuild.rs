//! Rebuilding the content index from the files (ADR 0001, ADR 0009).

use khalid_core::Slug;
use sqlx::{Sqlite, Transaction};

use crate::{ContentIndex, Database, DbError, SiteId};

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
    /// Returns [`DbError::Query`] when a row cannot be written.
    pub async fn rebuild(&self, site: SiteId, index: &ContentIndex) -> Result<(), DbError> {
        let ContentIndex {} = index;
        let mut tx = self.writer.begin().await?;
        clear(&mut tx, site).await?;
        tx.commit().await?;
        Ok(())
    }
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
