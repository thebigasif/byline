//! Post schedules on SQLite, for `khalid-api`'s publish door.

use chrono::{DateTime, SecondsFormat, Utc};
use khalid_api::content_store::BoxFuture;
use khalid_api::schedule_store::{Due, Schedule, ScheduleStore, ScheduleStoreError};
use sqlx::Row;

use crate::{Database, DbError, SiteId};

/// One site's post schedules in `post_schedules`. They are visitor data:
/// a rebuild keeps them, and a lost database leaves each scheduled post a
/// draft. Cloning shares the connections.
#[derive(Debug, Clone)]
pub struct ScheduleTable {
    db: Database,
    site: SiteId,
}

impl ScheduleTable {
    /// The schedules of `site` in `db`.
    pub fn new(db: Database, site: SiteId) -> Self {
        Self { db, site }
    }

    async fn get_now(&self, id: &str) -> Result<Schedule, DbError> {
        let row = sqlx::query(
            "SELECT publish_at, unpublish_at FROM post_schedules \
             WHERE site_id = ?1 AND post_id = ?2",
        )
        .bind(self.site.as_row())
        .bind(id)
        .fetch_optional(&self.db.readers)
        .await?;
        let Some(row) = row else {
            return Ok(Schedule::default());
        };
        let mut schedule = Schedule::default();
        if let Some(at) = time(row.try_get("publish_at")?)? {
            schedule = schedule.publishing_at(at);
        }
        if let Some(at) = time(row.try_get("unpublish_at")?)? {
            schedule = schedule.unpublishing_at(at);
        }
        Ok(schedule)
    }

    async fn set_now(&self, id: &str, schedule: Schedule) -> Result<(), DbError> {
        if schedule.is_empty() {
            sqlx::query("DELETE FROM post_schedules WHERE site_id = ?1 AND post_id = ?2")
                .bind(self.site.as_row())
                .bind(id)
                .execute(&self.db.writer)
                .await?;
            return Ok(());
        }
        sqlx::query(
            "INSERT INTO post_schedules (site_id, post_id, publish_at, unpublish_at) \
             VALUES (?1, ?2, ?3, ?4) ON CONFLICT (site_id, post_id) DO UPDATE SET \
             publish_at = excluded.publish_at, unpublish_at = excluded.unpublish_at",
        )
        .bind(self.site.as_row())
        .bind(id)
        .bind(schedule.publish_at().map(text))
        .bind(schedule.unpublish_at().map(text))
        .execute(&self.db.writer)
        .await?;
        Ok(())
    }

    async fn due_now(&self, now: DateTime<Utc>) -> Result<Vec<(String, Due)>, DbError> {
        let rows = sqlx::query(
            "SELECT post_id, 'publish' AS what, publish_at AS at FROM post_schedules \
             WHERE site_id = ?1 AND publish_at <= ?2 \
             UNION ALL \
             SELECT post_id, 'unpublish', unpublish_at FROM post_schedules \
             WHERE site_id = ?1 AND unpublish_at <= ?2 \
             ORDER BY at, post_id",
        )
        .bind(self.site.as_row())
        .bind(text(now))
        .fetch_all(&self.db.readers)
        .await?;
        rows.iter()
            .map(|row| {
                let (id, what): (String, String) = (row.try_get("post_id")?, row.try_get("what")?);
                let due = if what == "publish" {
                    Due::Publish
                } else {
                    Due::Unpublish
                };
                Ok((id, due))
            })
            .collect()
    }
}

impl ScheduleStore for ScheduleTable {
    fn get<'a>(&'a self, id: &'a str) -> BoxFuture<'a, Result<Schedule, ScheduleStoreError>> {
        Box::pin(async move { self.get_now(id).await.map_err(ScheduleStoreError::new) })
    }

    fn set<'a>(
        &'a self,
        id: &'a str,
        schedule: Schedule,
    ) -> BoxFuture<'a, Result<(), ScheduleStoreError>> {
        Box::pin(async move {
            self.set_now(id, schedule)
                .await
                .map_err(ScheduleStoreError::new)
        })
    }

    fn due(
        &self,
        now: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<Vec<(String, Due)>, ScheduleStoreError>> {
        Box::pin(async move { self.due_now(now).await.map_err(ScheduleStoreError::new) })
    }
}

/// `at` as the table writes it: RFC 3339 in UTC, to the second.
fn text(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// A stored time, read back.
fn time(stored: Option<String>) -> Result<Option<DateTime<Utc>>, DbError> {
    stored
        .map(|text| {
            DateTime::parse_from_rfc3339(&text)
                .map(|at| at.with_timezone(&Utc))
                .map_err(|_| DbError::Unreadable {
                    column: "post_schedules time",
                    value: text,
                })
        })
        .transpose()
}
