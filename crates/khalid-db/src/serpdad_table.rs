//! The `SerpDad` connection on SQLite, for the Admin API's `SerpDad` route
//! (KH-API-6).

use std::sync::Arc;

use chrono::{DateTime, SecondsFormat, Utc};
use khalid_api::content_store::BoxFuture;
use khalid_api::keys::KeyId;
use khalid_api::resources::LintCounts;
use khalid_api::serpdad_store::{
    ArrivedDraft, RECENT_DRAFTS, ReceivedDraft, SerpdadLink, SerpdadStore, SerpdadStoreError,
};
use khalid_core::Clock;
use sqlx::Row;
use sqlx::sqlite::SqliteRow;

use crate::{Database, DbError, SiteId};

/// One site's `SerpDad` key in `serpdad_links` and the drafts that arrived
/// with it in `serpdad_drafts`. Both are visitor data: a rebuild keeps
/// them. Cloning shares the connections.
#[derive(Debug, Clone)]
pub struct SerpdadTable {
    db: Database,
    site: SiteId,
    clock: Arc<dyn Clock>,
}

impl SerpdadTable {
    /// The connection of `site` in `db`, dating changes with `clock`.
    pub fn new(db: Database, site: SiteId, clock: impl Clock + 'static) -> Self {
        Self {
            db,
            site,
            clock: Arc::new(clock),
        }
    }

    fn now(&self) -> String {
        self.clock.now().to_rfc3339_opts(SecondsFormat::Secs, true)
    }

    async fn linked(&self) -> Result<Option<SerpdadLink>, DbError> {
        let row = sqlx::query("SELECT key_id, connected_at FROM serpdad_links WHERE site_id = ?1")
            .bind(self.site.as_row())
            .fetch_optional(&self.db.readers)
            .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let (key, at): (String, String) = (row.try_get("key_id")?, row.try_get("connected_at")?);
        let key = KeyId::try_new(&key).map_err(|_| unreadable("key_id", &key))?;
        Ok(Some(SerpdadLink::new(key, time("connected_at", &at)?)))
    }

    /// Replaces the link with `key`, or removes it, in one transaction, and
    /// answers the key it had.
    async fn relink(&self, key: Option<&KeyId>) -> Result<Option<KeyId>, DbError> {
        let mut transaction = self.db.writer.begin().await?;
        let old: Option<String> =
            sqlx::query_scalar("DELETE FROM serpdad_links WHERE site_id = ?1 RETURNING key_id")
                .bind(self.site.as_row())
                .fetch_optional(&mut *transaction)
                .await?;
        if let Some(key) = key {
            sqlx::query(
                "INSERT INTO serpdad_links (site_id, key_id, connected_at) VALUES (?1, ?2, ?3)",
            )
            .bind(self.site.as_row())
            .bind(key.as_str())
            .bind(self.now())
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        old.map(|old| KeyId::try_new(&old).map_err(|_| unreadable("key_id", &old)))
            .transpose()
    }

    /// Records `draft` and drops all but the newest [`RECENT_DRAFTS`].
    async fn add(&self, draft: &ArrivedDraft) -> Result<(), DbError> {
        let mut transaction = self.db.writer.begin().await?;
        sqlx::query(
            "INSERT INTO serpdad_drafts \
             (site_id, post_id, title, blocks, warnings, received_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind(self.site.as_row())
        .bind(&draft.post)
        .bind(&draft.title)
        .bind(draft.lint.map(LintCounts::blocks))
        .bind(draft.lint.map(LintCounts::warnings))
        .bind(self.now())
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            "DELETE FROM serpdad_drafts WHERE site_id = ?1 AND id NOT IN \
             (SELECT id FROM serpdad_drafts WHERE site_id = ?1 ORDER BY id DESC LIMIT ?2)",
        )
        .bind(self.site.as_row())
        .bind(RECENT_DRAFTS)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    async fn newest(&self) -> Result<Vec<ReceivedDraft>, DbError> {
        let rows = sqlx::query(
            "SELECT post_id, title, blocks, warnings, received_at FROM serpdad_drafts \
             WHERE site_id = ?1 ORDER BY id DESC LIMIT ?2",
        )
        .bind(self.site.as_row())
        .bind(RECENT_DRAFTS)
        .fetch_all(&self.db.readers)
        .await?;
        rows.iter().map(received).collect()
    }
}

impl SerpdadStore for SerpdadTable {
    fn link(&self) -> BoxFuture<'_, Result<Option<SerpdadLink>, SerpdadStoreError>> {
        Box::pin(async move { self.linked().await.map_err(SerpdadStoreError::storage) })
    }

    fn connect<'a>(
        &'a self,
        key: &'a KeyId,
    ) -> BoxFuture<'a, Result<Option<KeyId>, SerpdadStoreError>> {
        Box::pin(async move {
            self.relink(Some(key))
                .await
                .map_err(SerpdadStoreError::storage)
        })
    }

    fn disconnect(&self) -> BoxFuture<'_, Result<Option<KeyId>, SerpdadStoreError>> {
        Box::pin(async move { self.relink(None).await.map_err(SerpdadStoreError::storage) })
    }

    fn record<'a>(
        &'a self,
        draft: &'a ArrivedDraft,
    ) -> BoxFuture<'a, Result<(), SerpdadStoreError>> {
        Box::pin(async move { self.add(draft).await.map_err(SerpdadStoreError::storage) })
    }

    fn recent(&self) -> BoxFuture<'_, Result<Vec<ReceivedDraft>, SerpdadStoreError>> {
        Box::pin(async move { self.newest().await.map_err(SerpdadStoreError::storage) })
    }
}

fn received(row: &SqliteRow) -> Result<ReceivedDraft, DbError> {
    let blocks: Option<u32> = row.try_get("blocks")?;
    let warnings: Option<u32> = row.try_get("warnings")?;
    let lint = blocks.zip(warnings).map(|(b, w)| LintCounts::new(b, w));
    let draft = ArrivedDraft::new(row.try_get("post_id")?, row.try_get("title")?, lint);
    let at: String = row.try_get("received_at")?;
    Ok(ReceivedDraft::new(draft, time("received_at", &at)?))
}

fn time(column: &'static str, text: &str) -> Result<DateTime<Utc>, DbError> {
    DateTime::parse_from_rfc3339(text)
        .map(|at| at.to_utc())
        .map_err(|_| unreadable(column, text))
}

fn unreadable(column: &'static str, value: &str) -> DbError {
    DbError::Unreadable {
        column,
        value: value.to_owned(),
    }
}
