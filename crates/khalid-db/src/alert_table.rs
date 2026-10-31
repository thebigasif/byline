//! The alerts already raised on SQLite, for `khalid-alerts` (KH-GRO-6).

use std::collections::BTreeSet;

use chrono::{DateTime, SecondsFormat, Utc};
use khalid_alerts::{BoxFuture, Kind, Raised, RaisedError};

use crate::{Database, DbError, SiteId};

/// One site's raised alert keys in `alerts_raised`. It is visitor data: a
/// rebuild keeps it. Cloning shares the connections.
#[derive(Debug, Clone)]
pub struct AlertTable {
    db: Database,
    site: SiteId,
}

impl AlertTable {
    /// The raised alerts of `site` in `db`.
    pub fn new(db: Database, site: SiteId) -> Self {
        Self { db, site }
    }

    async fn raised_now(&self, kind: Kind) -> Result<BTreeSet<String>, DbError> {
        let prefix = kind.key("");
        let keys: Vec<String> = sqlx::query_scalar(
            "SELECT key FROM alerts_raised \
             WHERE site_id = ?1 AND substr(key, 1, length(?2)) = ?2",
        )
        .bind(self.site.as_row())
        .bind(&prefix)
        .fetch_all(&self.db.readers)
        .await?;
        Ok(keys.into_iter().collect())
    }

    async fn remember_now(&self, keys: &[String], at: DateTime<Utc>) -> Result<(), DbError> {
        let at = text(at);
        let mut transaction = self.db.writer.begin().await?;
        for key in keys {
            sqlx::query(
                "INSERT INTO alerts_raised (site_id, key, raised_at) VALUES (?1, ?2, ?3) \
                 ON CONFLICT (site_id, key) DO UPDATE SET raised_at = excluded.raised_at",
            )
            .bind(self.site.as_row())
            .bind(key)
            .bind(&at)
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    async fn forget_now(&self, keys: &[String]) -> Result<(), DbError> {
        let mut transaction = self.db.writer.begin().await?;
        for key in keys {
            sqlx::query("DELETE FROM alerts_raised WHERE site_id = ?1 AND key = ?2")
                .bind(self.site.as_row())
                .bind(key)
                .execute(&mut *transaction)
                .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    async fn forget_before_now(&self, before: DateTime<Utc>) -> Result<(), DbError> {
        sqlx::query("DELETE FROM alerts_raised WHERE site_id = ?1 AND raised_at < ?2")
            .bind(self.site.as_row())
            .bind(text(before))
            .execute(&self.db.writer)
            .await?;
        Ok(())
    }
}

impl Raised for AlertTable {
    fn raised(&self, kind: Kind) -> BoxFuture<'_, Result<BTreeSet<String>, RaisedError>> {
        Box::pin(async move { self.raised_now(kind).await.map_err(RaisedError::new) })
    }

    fn remember<'a>(
        &'a self,
        keys: &'a [String],
        at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<(), RaisedError>> {
        Box::pin(async move { self.remember_now(keys, at).await.map_err(RaisedError::new) })
    }

    fn forget<'a>(&'a self, keys: &'a [String]) -> BoxFuture<'a, Result<(), RaisedError>> {
        Box::pin(async move { self.forget_now(keys).await.map_err(RaisedError::new) })
    }

    fn forget_before(&self, before: DateTime<Utc>) -> BoxFuture<'_, Result<(), RaisedError>> {
        Box::pin(async move {
            self.forget_before_now(before)
                .await
                .map_err(RaisedError::new)
        })
    }
}

/// `at` as the table writes it: RFC 3339 in UTC, to the second, so it
/// sorts as text.
fn text(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(SecondsFormat::Secs, true)
}
