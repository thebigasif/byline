//! Earlier versions of posts on SQLite, for `khalid-api`'s publish door.

use chrono::{DateTime, SecondsFormat, Utc};
use khalid_api::content_store::BoxFuture;
use khalid_api::version_store::{MAX_VERSIONS, PostVersion, VersionStore, VersionStoreError};
use sqlx::Row;

use crate::{Database, DbError, SiteId};

/// One site's earlier post versions in `post_versions`. They are visitor
/// data: a rebuild keeps them. Cloning shares the connections.
#[derive(Debug, Clone)]
pub struct VersionTable {
    db: Database,
    site: SiteId,
}

impl VersionTable {
    /// The earlier versions of `site`'s posts in `db`.
    pub fn new(db: Database, site: SiteId) -> Self {
        Self { db, site }
    }

    async fn keep_now(&self, id: &str, version: &PostVersion) -> Result<(), DbError> {
        let mut transaction = self.db.writer.begin().await?;
        sqlx::query(
            "INSERT INTO post_versions \
             (site_id, post_id, kept, saved_at, saved_by, title, markdown) \
             SELECT ?1, ?2, coalesce(max(kept), 0) + 1, ?3, ?4, ?5, ?6 \
             FROM post_versions WHERE site_id = ?1 AND post_id = ?2",
        )
        .bind(self.site.as_row())
        .bind(id)
        .bind(
            version
                .saved_at()
                .to_rfc3339_opts(SecondsFormat::Secs, true),
        )
        .bind(version.saved_by())
        .bind(version.title())
        .bind(version.markdown())
        .execute(&mut *transaction)
        .await?;
        let newest = i64::try_from(MAX_VERSIONS).unwrap_or(i64::MAX);
        sqlx::query(
            "DELETE FROM post_versions WHERE site_id = ?1 AND post_id = ?2 AND kept <= \
             (SELECT max(kept) FROM post_versions WHERE site_id = ?1 AND post_id = ?2) - ?3",
        )
        .bind(self.site.as_row())
        .bind(id)
        .bind(newest)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    async fn list_now(&self, id: &str) -> Result<Vec<PostVersion>, DbError> {
        let rows = sqlx::query(
            "SELECT saved_at, saved_by, title, markdown FROM post_versions \
             WHERE site_id = ?1 AND post_id = ?2 ORDER BY kept DESC",
        )
        .bind(self.site.as_row())
        .bind(id)
        .fetch_all(&self.db.readers)
        .await?;
        rows.iter()
            .map(|row| {
                let saved_at: String = row.try_get("saved_at")?;
                let saved_at = DateTime::parse_from_rfc3339(&saved_at)
                    .map(|at| at.with_timezone(&Utc))
                    .map_err(|_| DbError::Unreadable {
                        column: "post_versions.saved_at",
                        value: saved_at.clone(),
                    })?;
                let saved_by: Option<String> = row.try_get("saved_by")?;
                let title: String = row.try_get("title")?;
                let markdown: String = row.try_get("markdown")?;
                Ok(PostVersion::new((saved_at, saved_by), title, markdown))
            })
            .collect()
    }
}

impl VersionStore for VersionTable {
    fn keep<'a>(
        &'a self,
        id: &'a str,
        version: PostVersion,
    ) -> BoxFuture<'a, Result<(), VersionStoreError>> {
        Box::pin(async move {
            self.keep_now(id, &version)
                .await
                .map_err(VersionStoreError::new)
        })
    }

    fn list<'a>(
        &'a self,
        id: &'a str,
    ) -> BoxFuture<'a, Result<Vec<PostVersion>, VersionStoreError>> {
        Box::pin(async move { self.list_now(id).await.map_err(VersionStoreError::new) })
    }
}
