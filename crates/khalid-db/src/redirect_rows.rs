//! Stored redirects on SQLite, for `khalid-api`'s redirect routes, the
//! command line and the server.

use chrono::{DateTime, SecondsFormat, Utc};
use khalid_api::content_store::BoxFuture;
use khalid_api::redirect_store::{
    Origin, RedirectStore, RedirectStoreError, StoredRedirect, source_of,
};
use khalid_seo::{Matching, RedirectColumns, RedirectEntry};
use sqlx::Row;
use sqlx::sqlite::SqliteRow;

use crate::{Database, DbError, SiteId};

/// One site's redirects in `redirects`. They are visitor data: a rebuild
/// keeps them. Cloning shares the connections.
#[derive(Debug, Clone)]
pub struct RedirectRows {
    db: Database,
    site: SiteId,
}

/// Why a batch was not stored.
enum AddError {
    Taken(String),
    Db(DbError),
}

impl From<sqlx::Error> for AddError {
    fn from(error: sqlx::Error) -> Self {
        Self::Db(error.into())
    }
}

impl RedirectRows {
    /// The redirects of `site` in `db`.
    pub fn new(db: Database, site: SiteId) -> Self {
        Self { db, site }
    }

    async fn list_now(&self) -> Result<Vec<StoredRedirect>, DbError> {
        let rows = sqlx::query(
            "SELECT id, from_path, to_url, status, match_kind, origin, created_at \
             FROM redirects WHERE site_id = ?1 ORDER BY id",
        )
        .bind(self.site.as_row())
        .fetch_all(&self.db.readers)
        .await?;
        rows.iter().map(stored).collect()
    }

    /// Inserts every entry in one transaction, which rolls back when an
    /// address is taken or a write fails.
    async fn add_now(
        &self,
        entries: Vec<RedirectEntry>,
        origin: Origin,
        at: DateTime<Utc>,
    ) -> Result<Vec<StoredRedirect>, AddError> {
        let mut transaction = self.db.writer.begin().await?;
        let mut added = Vec::with_capacity(entries.len());
        let created = at.to_rfc3339_opts(SecondsFormat::Secs, true);
        for entry in entries {
            let columns = entry.columns();
            let inserted = sqlx::query(
                "INSERT INTO redirects \
                 (site_id, from_path, to_url, status, match_kind, origin, created_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) RETURNING id",
            )
            .bind(self.site.as_row())
            .bind(&columns.from)
            .bind(columns.to.as_deref())
            .bind(i64::from(columns.status))
            .bind(columns.matching.as_str())
            .bind(origin.as_str())
            .bind(&created)
            .fetch_one(&mut *transaction)
            .await;
            let id: i64 = match inserted {
                Ok(row) => row.try_get("id")?,
                Err(error) if is_unique_violation(&error) => {
                    return Err(AddError::Taken(source_of(&entry).to_owned()));
                }
                Err(error) => return Err(error.into()),
            };
            added.push(StoredRedirect::new(id, entry, origin, at));
        }
        transaction.commit().await?;
        Ok(added)
    }

    async fn remove_now(&self, id: i64) -> Result<bool, DbError> {
        let gone = sqlx::query("DELETE FROM redirects WHERE site_id = ?1 AND id = ?2")
            .bind(self.site.as_row())
            .bind(id)
            .execute(&self.db.writer)
            .await?;
        Ok(gone.rows_affected() > 0)
    }
}

impl RedirectStore for RedirectRows {
    fn list(&self) -> BoxFuture<'_, Result<Vec<StoredRedirect>, RedirectStoreError>> {
        Box::pin(async move { self.list_now().await.map_err(RedirectStoreError::storage) })
    }

    fn add(
        &self,
        entries: Vec<RedirectEntry>,
        origin: Origin,
        at: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<Vec<StoredRedirect>, RedirectStoreError>> {
        Box::pin(async move {
            self.add_now(entries, origin, at)
                .await
                .map_err(|error| match error {
                    AddError::Taken(from) => RedirectStoreError::Taken { from },
                    AddError::Db(error) => RedirectStoreError::storage(error),
                })
        })
    }

    fn remove(&self, id: i64) -> BoxFuture<'_, Result<bool, RedirectStoreError>> {
        Box::pin(async move {
            self.remove_now(id)
                .await
                .map_err(RedirectStoreError::storage)
        })
    }
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .is_some_and(sqlx::error::DatabaseError::is_unique_violation)
}

/// One row read back as a checked redirect.
fn stored(row: &SqliteRow) -> Result<StoredRedirect, DbError> {
    let unreadable = |column: &'static str, value: String| DbError::Unreadable { column, value };
    let (from, to): (String, Option<String>) = (row.try_get("from_path")?, row.try_get("to_url")?);
    let status: i64 = row.try_get("status")?;
    let kind: String = row.try_get("match_kind")?;
    let matching = Matching::from_name(&kind).ok_or_else(|| unreadable("match_kind", kind))?;
    let status =
        u16::try_from(status).map_err(|_| unreadable("redirects.status", status.to_string()))?;
    let columns = RedirectColumns::new(&from, to.as_deref(), status, matching);
    let entry = RedirectEntry::from_columns(&columns)
        .map_err(|error| unreadable("redirects", error.to_string()))?;
    let origin: String = row.try_get("origin")?;
    let origin =
        Origin::from_name(&origin).ok_or_else(|| unreadable("redirects.origin", origin))?;
    let created: String = row.try_get("created_at")?;
    let created_at = DateTime::parse_from_rfc3339(&created)
        .map(|at| at.with_timezone(&Utc))
        .map_err(|_| unreadable("redirects.created_at", created))?;
    let id: i64 = row.try_get("id")?;
    Ok(StoredRedirect::new(id, entry, origin, created_at))
}
