//! The 404 log on SQLite, for `khalid-api`'s redirect routes and the
//! server that records each address not found.

use chrono::{DateTime, SecondsFormat, Utc};
use khalid_api::content_store::BoxFuture;
use khalid_api::not_found_log::{MAX_MISSES, Miss, NotFoundLog, NotFoundLogError};
use sqlx::Row;

use crate::{Database, DbError, SiteId};

/// One site's 404 log in `not_found_log`. It is visitor data: a rebuild
/// keeps it. Cloning shares the connections.
#[derive(Debug, Clone)]
pub struct NotFoundTable {
    db: Database,
    site: SiteId,
}

impl NotFoundTable {
    /// The 404 log of `site` in `db`.
    pub fn new(db: Database, site: SiteId) -> Self {
        Self { db, site }
    }

    async fn record_now(&self, path: &str, at: DateTime<Utc>) -> Result<(), DbError> {
        let at = text(at);
        let counted = sqlx::query(
            "UPDATE not_found_log SET hits = hits + 1, last_seen = ?3 \
             WHERE site_id = ?1 AND path = ?2",
        )
        .bind(self.site.as_row())
        .bind(path)
        .bind(&at)
        .execute(&self.db.writer)
        .await?;
        if counted.rows_affected() > 0 {
            return Ok(());
        }
        // A new address goes in only while the log has room.
        let limit = i64::try_from(MAX_MISSES).unwrap_or(i64::MAX);
        sqlx::query(
            "INSERT INTO not_found_log (site_id, path, hits, first_seen, last_seen) \
             SELECT ?1, ?2, 1, ?3, ?3 \
             WHERE (SELECT count(*) FROM not_found_log WHERE site_id = ?1) < ?4 \
             ON CONFLICT (site_id, path) DO UPDATE SET \
             hits = hits + 1, last_seen = excluded.last_seen",
        )
        .bind(self.site.as_row())
        .bind(path)
        .bind(&at)
        .bind(limit)
        .execute(&self.db.writer)
        .await?;
        Ok(())
    }

    async fn misses_now(&self, limit: u32) -> Result<Vec<Miss>, DbError> {
        let rows = sqlx::query(
            "SELECT path, hits, first_seen, last_seen FROM not_found_log \
             WHERE site_id = ?1 ORDER BY hits DESC, path LIMIT ?2",
        )
        .bind(self.site.as_row())
        .bind(i64::from(limit))
        .fetch_all(&self.db.readers)
        .await?;
        rows.iter()
            .map(|row| {
                let path: String = row.try_get("path")?;
                let hits: i64 = row.try_get("hits")?;
                let hits = u64::try_from(hits).map_err(|_| DbError::Unreadable {
                    column: "not_found_log.hits",
                    value: hits.to_string(),
                })?;
                let first_seen = time(row.try_get("first_seen")?)?;
                let last_seen = time(row.try_get("last_seen")?)?;
                Ok(Miss::new(&path, hits, first_seen, last_seen))
            })
            .collect()
    }

    async fn forget_now(&self, path: &str) -> Result<bool, DbError> {
        let gone = sqlx::query("DELETE FROM not_found_log WHERE site_id = ?1 AND path = ?2")
            .bind(self.site.as_row())
            .bind(path)
            .execute(&self.db.writer)
            .await?;
        Ok(gone.rows_affected() > 0)
    }
}

impl NotFoundLog for NotFoundTable {
    fn record<'a>(
        &'a self,
        path: &'a str,
        at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<(), NotFoundLogError>> {
        Box::pin(async move {
            self.record_now(path, at)
                .await
                .map_err(NotFoundLogError::new)
        })
    }

    fn misses(&self, limit: u32) -> BoxFuture<'_, Result<Vec<Miss>, NotFoundLogError>> {
        Box::pin(async move { self.misses_now(limit).await.map_err(NotFoundLogError::new) })
    }

    fn forget<'a>(&'a self, path: &'a str) -> BoxFuture<'a, Result<bool, NotFoundLogError>> {
        Box::pin(async move { self.forget_now(path).await.map_err(NotFoundLogError::new) })
    }
}

/// `at` as the table writes it: RFC 3339 in UTC, to the second.
fn text(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// A stored time, read back.
fn time(stored: String) -> Result<DateTime<Utc>, DbError> {
    DateTime::parse_from_rfc3339(&stored)
        .map(|at| at.with_timezone(&Utc))
        .map_err(|_| DbError::Unreadable {
            column: "not_found_log time",
            value: stored,
        })
}
