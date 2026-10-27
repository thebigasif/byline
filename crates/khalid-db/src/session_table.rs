//! Signed-in sessions on SQLite, for `khalid-auth`.

use chrono::{DateTime, SecondsFormat, Utc};
use khalid_api::content_store::BoxFuture;
use khalid_auth::{
    AccountId, LIFETIME, SessionHash, SessionStore, SessionStoreError, StoredSession,
};
use sqlx::Row;

use crate::{Database, DbError, SiteId};

/// One site's sessions in `sessions`, each stored as the SHA-256 of its
/// token, never the token (RULES.md BL-SEC-3). Cloning shares the
/// connections.
#[derive(Debug, Clone)]
pub struct SessionTable {
    db: Database,
    site: SiteId,
}

impl SessionTable {
    /// The sessions of `site` in `db`.
    pub fn new(db: Database, site: SiteId) -> Self {
        Self { db, site }
    }

    async fn begin_now(
        &self,
        hash: SessionHash,
        account: AccountId,
        at: DateTime<Utc>,
    ) -> Result<(), DbError> {
        sqlx::query(
            "INSERT INTO sessions (site_id, user_id, token_hash, created_at, expires_at, last_seen_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?4)",
        )
        .bind(self.site.as_row())
        .bind(account.as_row())
        .bind(hash.as_bytes().as_slice())
        .bind(text(at))
        .bind(text(at + LIFETIME))
        .execute(&self.db.writer)
        .await?;
        Ok(())
    }

    async fn find_now(&self, hash: SessionHash) -> Result<Option<StoredSession>, DbError> {
        let row = sqlx::query(
            "SELECT user_id, created_at, last_seen_at FROM sessions \
             WHERE site_id = ?1 AND token_hash = ?2",
        )
        .bind(self.site.as_row())
        .bind(hash.as_bytes().as_slice())
        .fetch_optional(&self.db.readers)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let created_at = time(&row.try_get::<String, _>("created_at")?)?;
        let last_seen_at = match row.try_get::<Option<String>, _>("last_seen_at")? {
            Some(seen) => time(&seen)?,
            None => created_at,
        };
        Ok(Some(StoredSession {
            account: AccountId::from_row(row.try_get("user_id")?),
            created_at,
            last_seen_at,
        }))
    }

    async fn touch_now(&self, hash: SessionHash, at: DateTime<Utc>) -> Result<(), DbError> {
        sqlx::query("UPDATE sessions SET last_seen_at = ?1 WHERE site_id = ?2 AND token_hash = ?3")
            .bind(text(at))
            .bind(self.site.as_row())
            .bind(hash.as_bytes().as_slice())
            .execute(&self.db.writer)
            .await?;
        Ok(())
    }

    async fn end_now(&self, hash: SessionHash) -> Result<bool, DbError> {
        let done = sqlx::query("DELETE FROM sessions WHERE site_id = ?1 AND token_hash = ?2")
            .bind(self.site.as_row())
            .bind(hash.as_bytes().as_slice())
            .execute(&self.db.writer)
            .await?;
        Ok(done.rows_affected() > 0)
    }

    async fn end_all_now(
        &self,
        account: AccountId,
        keep: Option<SessionHash>,
    ) -> Result<u64, DbError> {
        let done = sqlx::query(
            "DELETE FROM sessions WHERE site_id = ?1 AND user_id = ?2 \
             AND (?3 IS NULL OR token_hash != ?3)",
        )
        .bind(self.site.as_row())
        .bind(account.as_row())
        .bind(keep.as_ref().map(|hash| hash.as_bytes().to_vec()))
        .execute(&self.db.writer)
        .await?;
        Ok(done.rows_affected())
    }
}

fn text(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn time(text: &str) -> Result<DateTime<Utc>, DbError> {
    DateTime::parse_from_rfc3339(text)
        .map(|at| at.with_timezone(&Utc))
        .map_err(|_| DbError::Unreadable {
            column: "sessions time",
            value: text.to_owned(),
        })
}

impl SessionStore for SessionTable {
    fn begin(
        &self,
        hash: SessionHash,
        account: AccountId,
        at: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<(), SessionStoreError>> {
        Box::pin(async move {
            self.begin_now(hash, account, at)
                .await
                .map_err(SessionStoreError::new)
        })
    }

    fn find(
        &self,
        hash: SessionHash,
    ) -> BoxFuture<'_, Result<Option<StoredSession>, SessionStoreError>> {
        Box::pin(async move { self.find_now(hash).await.map_err(SessionStoreError::new) })
    }

    fn touch(
        &self,
        hash: SessionHash,
        at: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<(), SessionStoreError>> {
        Box::pin(async move {
            self.touch_now(hash, at)
                .await
                .map_err(SessionStoreError::new)
        })
    }

    fn end(&self, hash: SessionHash) -> BoxFuture<'_, Result<bool, SessionStoreError>> {
        Box::pin(async move { self.end_now(hash).await.map_err(SessionStoreError::new) })
    }

    fn end_all(
        &self,
        account: AccountId,
        keep: Option<SessionHash>,
    ) -> BoxFuture<'_, Result<u64, SessionStoreError>> {
        Box::pin(async move {
            self.end_all_now(account, keep)
                .await
                .map_err(SessionStoreError::new)
        })
    }
}
