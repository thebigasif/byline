//! The speed and uptime watch's looks on SQLite, for `khalid serve` to
//! write and `khalid watch` to read (C6).

use chrono::{DateTime, SecondsFormat, Utc};
use khalid_alerts::{Answer, Probe};

use crate::{Database, DbError, SiteId};

/// One site's looks at its pages in `site_watch`. It is visitor data: a
/// rebuild keeps it. Cloning shares the connections.
#[derive(Debug, Clone)]
pub struct WatchTable {
    db: Database,
    site: SiteId,
}

impl WatchTable {
    /// The looks of `site` in `db`.
    pub fn new(db: Database, site: SiteId) -> Self {
        Self { db, site }
    }

    /// Keeps `probe`, a look at `page`. A second look at the same second
    /// replaces the first.
    ///
    /// # Errors
    ///
    /// [`DbError::Query`] when the row cannot be written.
    pub async fn record(&self, page: &str, probe: &Probe) -> Result<(), DbError> {
        let status = match probe.answer {
            Answer::Status(status) => Some(status),
            Answer::Unreached => None,
        };
        sqlx::query(
            "INSERT INTO site_watch (site_id, page, looked_at, status, millis) \
             VALUES (?1, ?2, ?3, ?4, ?5) \
             ON CONFLICT (site_id, page, looked_at) \
             DO UPDATE SET status = excluded.status, millis = excluded.millis",
        )
        .bind(self.site.as_row())
        .bind(page)
        .bind(text(probe.at))
        .bind(status)
        .bind(probe.millis)
        .execute(&self.db.writer)
        .await?;
        Ok(())
    }

    /// The looks at `page` from `since` on, oldest first.
    ///
    /// # Errors
    ///
    /// [`DbError::Query`] when the rows cannot be read, and
    /// [`DbError::Unreadable`] when a row was changed outside Khalid.
    pub async fn since(&self, page: &str, since: DateTime<Utc>) -> Result<Vec<Probe>, DbError> {
        let rows: Vec<(String, Option<i64>, i64)> = sqlx::query_as(
            "SELECT looked_at, status, millis FROM site_watch \
             WHERE site_id = ?1 AND page = ?2 AND looked_at >= ?3 ORDER BY looked_at",
        )
        .bind(self.site.as_row())
        .bind(page)
        .bind(text(since))
        .fetch_all(&self.db.readers)
        .await?;
        rows.into_iter().map(probe).collect()
    }

    /// Every page with a look kept, in order.
    ///
    /// # Errors
    ///
    /// [`DbError::Query`] when the rows cannot be read.
    pub async fn pages(&self) -> Result<Vec<String>, DbError> {
        let pages = sqlx::query_scalar(
            "SELECT DISTINCT page FROM site_watch WHERE site_id = ?1 ORDER BY page",
        )
        .bind(self.site.as_row())
        .fetch_all(&self.db.readers)
        .await?;
        Ok(pages)
    }

    /// Drops every look taken before `before`, so the table stays small.
    ///
    /// # Errors
    ///
    /// [`DbError::Query`] when the rows cannot be deleted.
    pub async fn forget_before(&self, before: DateTime<Utc>) -> Result<(), DbError> {
        sqlx::query("DELETE FROM site_watch WHERE site_id = ?1 AND looked_at < ?2")
            .bind(self.site.as_row())
            .bind(text(before))
            .execute(&self.db.writer)
            .await?;
        Ok(())
    }
}

/// A row read back as the look it was written from.
fn probe((at, status, millis): (String, Option<i64>, i64)) -> Result<Probe, DbError> {
    let unreadable = |column, value: String| DbError::Unreadable { column, value };
    let looked = DateTime::parse_from_rfc3339(&at).map_err(|_| unreadable("looked_at", at))?;
    let answer = match status {
        Some(status) => Answer::Status(
            u16::try_from(status).map_err(|_| unreadable("status", status.to_string()))?,
        ),
        None => Answer::Unreached,
    };
    let millis = u32::try_from(millis).map_err(|_| unreadable("millis", millis.to_string()))?;
    Ok(Probe {
        at: looked.to_utc(),
        answer,
        millis,
    })
}

/// `at` as the table writes it: RFC 3339 in UTC, to the second, so it
/// sorts as text.
fn text(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(SecondsFormat::Secs, true)
}
