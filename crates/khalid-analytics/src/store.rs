//! Where the counts are kept: `khalid-db` keeps them in SQLite as visitor
//! data.

use std::error::Error;
use std::future::Future;
use std::pin::Pin;

use chrono::{DateTime, NaiveDate, Utc};

use crate::{CrawledPage, Crawls, Tally, Views, Visit};

/// A boxed future, so the store traits stay object safe.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// The most crawl rows kept for one day. Past it, rows already stored are
/// still counted and new ones are dropped, so a bot trying made-up
/// addresses cannot grow the database without end.
pub const MAX_CRAWL_ROWS_PER_DAY: usize = 10_000;

/// How many days the crawl log keeps. Views are kept for good: a row per
/// page, day and group is small.
pub const CRAWL_DAYS_KEPT: u32 = 90;

/// The site's counts.
pub trait AnalyticsStore: Send + Sync {
    /// Adds every count in `tally` to the stored ones, and drops crawl
    /// rows more than [`CRAWL_DAYS_KEPT`] days older than its latest day.
    ///
    /// # Errors
    ///
    /// [`AnalyticsError`] when the counts cannot be written.
    fn save<'a>(&'a self, tally: &'a Tally) -> BoxFuture<'a, Result<(), AnalyticsError>>;

    /// The views from `first` to `last`, both days included, by day, then
    /// path, then group.
    ///
    /// # Errors
    ///
    /// [`AnalyticsError`] when the counts cannot be read.
    fn views(
        &self,
        first: NaiveDate,
        last: NaiveDate,
    ) -> BoxFuture<'_, Result<Vec<Views>, AnalyticsError>>;

    /// The crawls from `first` to `last`, both days included, by day, then
    /// path, then bot.
    ///
    /// # Errors
    ///
    /// [`AnalyticsError`] when the crawl log cannot be read.
    fn crawls(
        &self,
        first: NaiveDate,
        last: NaiveDate,
    ) -> BoxFuture<'_, Result<Vec<Crawls>, AnalyticsError>>;
}

/// The most visits the visitor log keeps for one day. Past it, new visits
/// are dropped, so a flood of requests cannot fill the disk.
pub const MAX_VISITS_PER_DAY: usize = 100_000;

/// The site's visitor log, one row per [`Visit`], kept a few days.
pub trait VisitorLogStore: Send + Sync {
    /// Adds `visits`, up to [`MAX_VISITS_PER_DAY`] for each day, and
    /// erases every visit that came before `keep_from`. Each
    /// [`CrawledPage`] in `visits` is kept too, and never erased: its first
    /// fetch never moves later, and its last never earlier.
    ///
    /// # Errors
    ///
    /// [`AnalyticsError`] when the log cannot be written.
    fn save<'a>(
        &'a self,
        visits: &'a [Visit],
        keep_from: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<(), AnalyticsError>>;

    /// The visits from `first` to `last`, both included, oldest first.
    ///
    /// # Errors
    ///
    /// [`AnalyticsError`] when the log cannot be read, or holds a row this
    /// version cannot read.
    fn visits(
        &self,
        first: DateTime<Utc>,
        last: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<Vec<Visit>, AnalyticsError>>;

    /// Every page a listed bot fetched while the log was on, by page then
    /// bot.
    ///
    /// # Errors
    ///
    /// [`AnalyticsError`] when the pages cannot be read, or hold a row this
    /// version cannot read.
    fn crawled(&self) -> BoxFuture<'_, Result<Vec<CrawledPage>, AnalyticsError>>;
}

/// The counts could not be read or written.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AnalyticsError {
    /// The storage failed, with what it reported.
    #[error("the site's analytics could not be read or written")]
    Storage(#[source] Box<dyn Error + Send + Sync>),
}
