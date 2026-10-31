//! The audit log on SQLite, for `khalid-api`'s audit routes and the key
//! check that records each change.

use chrono::{DateTime, SecondsFormat, Utc};
use khalid_api::audit_log::{Action, AuditLog, AuditLogError, Change, Logged, MAX_CHANGES};
use khalid_api::content_store::BoxFuture;
use sqlx::Row;
use sqlx::sqlite::SqliteRow;

use crate::{Database, DbError, SiteId};

/// One site's audit log in `audit_log`. It is visitor data: a rebuild
/// keeps it. Cloning shares the connections.
#[derive(Debug, Clone)]
pub struct AuditTable {
    db: Database,
    site: SiteId,
}

impl AuditTable {
    /// The audit log of `site` in `db`.
    pub fn new(db: Database, site: SiteId) -> Self {
        Self { db, site }
    }

    async fn record_now(&self, change: &Change) -> Result<(), DbError> {
        let at = change.at.to_rfc3339_opts(SecondsFormat::Secs, true);
        let mut writing = self.db.writer.begin().await?;
        sqlx::query(
            "INSERT INTO audit_log (site_id, at, by_key, action, route, path) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind(self.site.as_row())
        .bind(at)
        .bind(&change.by)
        .bind(change.action.as_str())
        .bind(&change.route)
        .bind(&change.path)
        .execute(&mut *writing)
        .await?;
        // The oldest rows past the limit go, so the log stays bounded.
        sqlx::query(
            "DELETE FROM audit_log WHERE site_id = ?1 AND id <= \
             (SELECT id FROM audit_log WHERE site_id = ?1 ORDER BY id DESC LIMIT 1 OFFSET ?2)",
        )
        .bind(self.site.as_row())
        .bind(i64::from(MAX_CHANGES))
        .execute(&mut *writing)
        .await?;
        writing.commit().await?;
        Ok(())
    }

    async fn changes_now(&self, before: Option<i64>, limit: u32) -> Result<Vec<Logged>, DbError> {
        let rows = sqlx::query(
            "SELECT id, at, by_key, action, route, path FROM audit_log \
             WHERE site_id = ?1 AND id < ?2 ORDER BY id DESC LIMIT ?3",
        )
        .bind(self.site.as_row())
        .bind(before.unwrap_or(i64::MAX))
        .bind(i64::from(limit))
        .fetch_all(&self.db.readers)
        .await?;
        rows.iter().map(logged).collect()
    }
}

impl AuditLog for AuditTable {
    fn record<'a>(&'a self, change: &'a Change) -> BoxFuture<'a, Result<(), AuditLogError>> {
        Box::pin(async move { self.record_now(change).await.map_err(AuditLogError::new) })
    }

    fn changes(
        &self,
        before: Option<i64>,
        limit: u32,
    ) -> BoxFuture<'_, Result<Vec<Logged>, AuditLogError>> {
        Box::pin(async move {
            self.changes_now(before, limit)
                .await
                .map_err(AuditLogError::new)
        })
    }
}

/// One stored row, read back.
fn logged(row: &SqliteRow) -> Result<Logged, DbError> {
    let at: String = row.try_get("at")?;
    let at = DateTime::parse_from_rfc3339(&at)
        .map(|at| at.with_timezone(&Utc))
        .map_err(|_| DbError::Unreadable {
            column: "audit_log.at",
            value: at.clone(),
        })?;
    let action: String = row.try_get("action")?;
    let action = Action::parse(&action).ok_or(DbError::Unreadable {
        column: "audit_log.action",
        value: action.clone(),
    })?;
    let by: String = row.try_get("by_key")?;
    let route: String = row.try_get("route")?;
    let path: String = row.try_get("path")?;
    let change = Change::new(at, &by, action, &route, &path);
    Ok(Logged::new(row.try_get("id")?, change))
}
