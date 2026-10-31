//! The site's analytics on SQLite: page views in `analytics_counts` and
//! the crawl log in `crawl_log`, written by `khalid serve` in batches.

use chrono::{DateTime, Days, NaiveDate, SecondsFormat, Utc};
use khalid_analytics::{
    AnalyticsError, AnalyticsStore, BoxFuture, CRAWL_DAYS_KEPT, CrawlerKind, Crawls,
    MAX_CRAWL_ROWS_PER_DAY, ReferrerGroup, Tally, Views,
};
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, Sqlite, Transaction};

use crate::{Database, DbError, SiteId};

/// The widest days a read asks for: wider ones would not sort as text.
const FIRST_DAY: NaiveDate = match NaiveDate::from_ymd_opt(1, 1, 1) {
    Some(day) => day,
    None => NaiveDate::MIN,
};
const LAST_DAY: NaiveDate = match NaiveDate::from_ymd_opt(9999, 12, 31) {
    Some(day) => day,
    None => NaiveDate::MAX,
};

/// One site's views and crawl log. Both are visitor data: a rebuild keeps
/// them. Cloning shares the connections.
#[derive(Debug, Clone)]
pub struct AnalyticsTable {
    db: Database,
    site: SiteId,
}

impl AnalyticsTable {
    /// The analytics of `site` in `db`.
    pub fn new(db: Database, site: SiteId) -> Self {
        Self { db, site }
    }

    async fn save_now(&self, tally: &Tally) -> Result<(), DbError> {
        let mut tx = self.db.writer.begin().await?;
        for row in tally.views() {
            sqlx::query(
                "INSERT INTO analytics_counts (site_id, day, path, referrer_group, views) \
                 VALUES (?1, ?2, ?3, ?4, ?5) \
                 ON CONFLICT (site_id, day, path, referrer_group) \
                 DO UPDATE SET views = views + excluded.views",
            )
            .bind(self.site.as_row())
            .bind(day_text(row.day))
            .bind(&row.path)
            .bind(row.group.as_str())
            .bind(count(row.views))
            .execute(&mut *tx)
            .await?;
        }
        let crawls = tally.crawls();
        for row in &crawls {
            if !self.add_crawl(&mut tx, row).await? {
                self.insert_crawl(&mut tx, row).await?;
            }
        }
        let latest = crawls.iter().map(|row| row.day).max();
        let kept = Days::new(u64::from(CRAWL_DAYS_KEPT));
        if let Some(oldest) = latest.and_then(|day| day.checked_sub_days(kept)) {
            sqlx::query("DELETE FROM crawl_log WHERE site_id = ?1 AND day < ?2")
                .bind(self.site.as_row())
                .bind(day_text(oldest))
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// Adds `row` to a stored row, answering whether there was one.
    async fn add_crawl(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        row: &Crawls,
    ) -> Result<bool, DbError> {
        let added = sqlx::query(
            "UPDATE crawl_log SET hits = hits + ?5, \
             last_status = CASE WHEN ?6 >= last_seen THEN ?7 ELSE last_status END, \
             last_seen = max(last_seen, ?6) \
             WHERE site_id = ?1 AND day = ?2 AND path = ?3 AND crawler = ?4",
        )
        .bind(self.site.as_row())
        .bind(day_text(row.day))
        .bind(&row.path)
        .bind(&row.crawler)
        .bind(count(row.hits))
        .bind(time_text(row.last_seen))
        .bind(i64::from(row.last_status))
        .execute(&mut **tx)
        .await?;
        Ok(added.rows_affected() > 0)
    }

    /// Stores `row` as a new row, while its day has room.
    async fn insert_crawl(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        row: &Crawls,
    ) -> Result<(), DbError> {
        let limit = i64::try_from(MAX_CRAWL_ROWS_PER_DAY).unwrap_or(i64::MAX);
        sqlx::query(
            "INSERT INTO crawl_log \
             (site_id, day, path, crawler, kind, hits, last_seen, last_status) \
             SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8 \
             WHERE (SELECT count(*) FROM crawl_log WHERE site_id = ?1 AND day = ?2) < ?9",
        )
        .bind(self.site.as_row())
        .bind(day_text(row.day))
        .bind(&row.path)
        .bind(&row.crawler)
        .bind(row.kind.as_str())
        .bind(count(row.hits))
        .bind(time_text(row.last_seen))
        .bind(i64::from(row.last_status))
        .bind(limit)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    async fn views_now(&self, first: NaiveDate, last: NaiveDate) -> Result<Vec<Views>, DbError> {
        let rows = sqlx::query(
            "SELECT day, path, referrer_group, views FROM analytics_counts \
             WHERE site_id = ?1 AND day BETWEEN ?2 AND ?3 ORDER BY day, path, referrer_group",
        )
        .bind(self.site.as_row())
        .bind(day_text(first.max(FIRST_DAY)))
        .bind(day_text(last.min(LAST_DAY)))
        .fetch_all(&self.db.readers)
        .await?;
        let mut views: Vec<Views> = rows.iter().map(view_row).collect::<Result<_, _>>()?;
        // Sorted by group order, which is not the order of the names.
        views.sort_by(|a, b| (a.day, &a.path, a.group).cmp(&(b.day, &b.path, b.group)));
        Ok(views)
    }

    async fn crawls_now(&self, first: NaiveDate, last: NaiveDate) -> Result<Vec<Crawls>, DbError> {
        let rows = sqlx::query(
            "SELECT day, path, crawler, kind, hits, last_seen, last_status FROM crawl_log \
             WHERE site_id = ?1 AND day BETWEEN ?2 AND ?3 ORDER BY day, path, crawler",
        )
        .bind(self.site.as_row())
        .bind(day_text(first.max(FIRST_DAY)))
        .bind(day_text(last.min(LAST_DAY)))
        .fetch_all(&self.db.readers)
        .await?;
        rows.iter().map(crawl_row).collect()
    }
}

impl AnalyticsStore for AnalyticsTable {
    fn save<'a>(&'a self, tally: &'a Tally) -> BoxFuture<'a, Result<(), AnalyticsError>> {
        Box::pin(async move { self.save_now(tally).await.map_err(failed) })
    }

    fn views(
        &self,
        first: NaiveDate,
        last: NaiveDate,
    ) -> BoxFuture<'_, Result<Vec<Views>, AnalyticsError>> {
        Box::pin(async move { self.views_now(first, last).await.map_err(failed) })
    }

    fn crawls(
        &self,
        first: NaiveDate,
        last: NaiveDate,
    ) -> BoxFuture<'_, Result<Vec<Crawls>, AnalyticsError>> {
        Box::pin(async move { self.crawls_now(first, last).await.map_err(failed) })
    }
}

fn failed(error: DbError) -> AnalyticsError {
    AnalyticsError::Storage(Box::new(error))
}

fn view_row(row: &SqliteRow) -> Result<Views, DbError> {
    let group: String = row.try_get("referrer_group")?;
    let group = ReferrerGroup::from_stored(&group).ok_or(DbError::Unreadable {
        column: "analytics_counts.referrer_group",
        value: group,
    })?;
    let (path, views): (String, i64) = (row.try_get("path")?, row.try_get("views")?);
    let day = day(row.try_get("day")?)?;
    Ok(Views::new(day, &path, group, number(views)?))
}

fn crawl_row(row: &SqliteRow) -> Result<Crawls, DbError> {
    let kind: String = row.try_get("kind")?;
    let kind = CrawlerKind::from_stored(&kind).ok_or(DbError::Unreadable {
        column: "crawl_log.kind",
        value: kind,
    })?;
    let status: i64 = row.try_get("last_status")?;
    let last_status = u16::try_from(status).map_err(|_| DbError::Unreadable {
        column: "crawl_log.last_status",
        value: status.to_string(),
    })?;
    Ok(Crawls {
        day: day(row.try_get("day")?)?,
        path: row.try_get("path")?,
        crawler: row.try_get("crawler")?,
        kind,
        hits: number(row.try_get("hits")?)?,
        last_seen: time(row.try_get("last_seen")?)?,
        last_status,
    })
}

/// A count as SQLite stores it.
fn count(count: u64) -> i64 {
    i64::try_from(count).unwrap_or(i64::MAX)
}

/// A stored count, read back.
fn number(stored: i64) -> Result<u64, DbError> {
    u64::try_from(stored).map_err(|_| DbError::Unreadable {
        column: "analytics count",
        value: stored.to_string(),
    })
}

fn day_text(day: NaiveDate) -> String {
    day.format("%Y-%m-%d").to_string()
}

fn day(stored: String) -> Result<NaiveDate, DbError> {
    NaiveDate::parse_from_str(&stored, "%Y-%m-%d").map_err(|_| DbError::Unreadable {
        column: "analytics day",
        value: stored,
    })
}

/// `at` as the table writes it: RFC 3339 in UTC, to the second, so text
/// order is time order.
fn time_text(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn time(stored: String) -> Result<DateTime<Utc>, DbError> {
    DateTime::parse_from_rfc3339(&stored)
        .map(|at| at.with_timezone(&Utc))
        .map_err(|_| DbError::Unreadable {
            column: "crawl_log.last_seen",
            value: stored,
        })
}
