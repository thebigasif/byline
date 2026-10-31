//! The visitor log on SQLite, in `visitor_log`, written by `khalid serve`
//! in batches while `[visitor_log]` is on (GEO C1).

use std::collections::BTreeMap;

use chrono::{DateTime, Datelike, NaiveDate, SecondsFormat, Utc};
use khalid_analytics::{
    AnalyticsError, BotCheck, BoxFuture, Crawler, MAX_VISITS_PER_DAY, ReferrerGroup, Visit,
    Visitor, VisitorLogStore,
};
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, Sqlite, Transaction};

use crate::{Database, DbError, SiteId};

/// One site's visitor log. Visitor data: a rebuild keeps it. Cloning
/// shares the connections.
#[derive(Debug, Clone)]
pub struct VisitorLogTable {
    db: Database,
    site: SiteId,
}

impl VisitorLogTable {
    /// The visitor log of `site` in `db`.
    pub fn new(db: Database, site: SiteId) -> Self {
        Self { db, site }
    }

    async fn save_now(&self, visits: &[Visit], keep_from: DateTime<Utc>) -> Result<(), DbError> {
        let mut by_day: BTreeMap<NaiveDate, Vec<&Visit>> = BTreeMap::new();
        for visit in visits {
            by_day.entry(visit.at.date_naive()).or_default().push(visit);
        }
        let mut tx = self.db.writer.begin().await?;
        sqlx::query("DELETE FROM visitor_log WHERE site_id = ?1 AND at < ?2")
            .bind(self.site.as_row())
            .bind(time_text(keep_from))
            .execute(&mut *tx)
            .await?;
        for (day, visits) in by_day {
            let room = MAX_VISITS_PER_DAY.saturating_sub(self.stored_on(&mut tx, day).await?);
            for visit in visits.into_iter().take(room) {
                self.insert(&mut tx, visit).await?;
            }
        }
        tx.commit().await?;
        Ok(())
    }

    /// How many visits are stored for `day`.
    async fn stored_on(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        day: NaiveDate,
    ) -> Result<usize, DbError> {
        let next = day.succ_opt().unwrap_or(day);
        let stored: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM visitor_log WHERE site_id = ?1 AND at >= ?2 AND at < ?3",
        )
        .bind(self.site.as_row())
        .bind(day.format("%Y-%m-%d").to_string())
        .bind(next.format("%Y-%m-%d").to_string())
        .fetch_one(&mut **tx)
        .await?;
        Ok(usize::try_from(stored).unwrap_or(MAX_VISITS_PER_DAY))
    }

    async fn insert(&self, tx: &mut Transaction<'_, Sqlite>, visit: &Visit) -> Result<(), DbError> {
        let (group, crawler, check) = match visit.visitor {
            Visitor::Person(group) => (Some(group.as_str()), None, None),
            Visitor::Bot(bot, check) => (None, Some(bot.name()), Some(check.as_str())),
        };
        sqlx::query(
            "INSERT INTO visitor_log (site_id, at, path, status, address, visitor_code, \
             referrer_group, crawler, bot_check) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )
        .bind(self.site.as_row())
        .bind(time_text(visit.at))
        .bind(&visit.path)
        .bind(i64::from(visit.status))
        .bind(visit.address.to_string())
        .bind(visit.code.to_string())
        .bind(group)
        .bind(crawler)
        .bind(check)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    async fn visits_now(
        &self,
        first: DateTime<Utc>,
        last: DateTime<Utc>,
    ) -> Result<Vec<Visit>, DbError> {
        let rows = sqlx::query(
            "SELECT at, path, status, address, visitor_code, referrer_group, crawler, bot_check \
             FROM visitor_log WHERE site_id = ?1 AND at BETWEEN ?2 AND ?3 ORDER BY at, rowid",
        )
        .bind(self.site.as_row())
        .bind(time_text(first))
        .bind(time_text(last))
        .fetch_all(&self.db.readers)
        .await?;
        rows.iter().map(visit_row).collect()
    }
}

impl VisitorLogStore for VisitorLogTable {
    fn save<'a>(
        &'a self,
        visits: &'a [Visit],
        keep_from: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<(), AnalyticsError>> {
        Box::pin(async move { self.save_now(visits, keep_from).await.map_err(failed) })
    }

    fn visits(
        &self,
        first: DateTime<Utc>,
        last: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<Vec<Visit>, AnalyticsError>> {
        Box::pin(async move { self.visits_now(first, last).await.map_err(failed) })
    }
}

fn failed(error: DbError) -> AnalyticsError {
    AnalyticsError::Storage(Box::new(error))
}

/// `at` as the table writes it: RFC 3339 in UTC, to the second, so text
/// order is time order. A time before year 1 or after 9999 is written as
/// that year's end, since a sign before the year would not sort as text.
fn time_text(at: DateTime<Utc>) -> String {
    match at.year() {
        ..1 => "0001-01-01T00:00:00Z".to_owned(),
        10_000.. => "9999-12-31T23:59:59Z".to_owned(),
        _ => at.to_rfc3339_opts(SecondsFormat::Secs, true),
    }
}

/// A stored value this version cannot read, named by its column.
fn unreadable(column: &'static str, value: &impl ToString) -> DbError {
    DbError::Unreadable {
        column,
        value: value.to_string(),
    }
}

fn visit_row(row: &SqliteRow) -> Result<Visit, DbError> {
    let at: String = row.try_get("at")?;
    let at = DateTime::parse_from_rfc3339(&at)
        .map_err(|_| unreadable("visitor_log.at", &at))?
        .with_timezone(&Utc);
    let status: i64 = row.try_get("status")?;
    let status = u16::try_from(status).map_err(|_| unreadable("visitor_log.status", &status))?;
    let address: String = row.try_get("address")?;
    let address = address
        .parse()
        .map_err(|_| unreadable("visitor_log.address", &address))?;
    let code: String = row.try_get("visitor_code")?;
    let code = code
        .parse()
        .map_err(|_| unreadable("visitor_log.visitor_code", &code))?;
    Ok(Visit {
        at,
        path: row.try_get("path")?,
        status,
        address,
        code,
        visitor: visitor(row)?,
    })
}

fn visitor(row: &SqliteRow) -> Result<Visitor, DbError> {
    let crawler: Option<String> = row.try_get("crawler")?;
    let Some(crawler) = crawler else {
        let group: Option<String> = row.try_get("referrer_group")?;
        let group = group.unwrap_or_default();
        return ReferrerGroup::from_stored(&group)
            .map(Visitor::Person)
            .ok_or_else(|| unreadable("visitor_log.referrer_group", &group));
    };
    let bot =
        Crawler::named(&crawler).ok_or_else(|| unreadable("visitor_log.crawler", &crawler))?;
    let check: Option<String> = row.try_get("bot_check")?;
    let check = check.unwrap_or_default();
    let check =
        BotCheck::from_stored(&check).ok_or_else(|| unreadable("visitor_log.bot_check", &check))?;
    Ok(Visitor::Bot(bot, check))
}
