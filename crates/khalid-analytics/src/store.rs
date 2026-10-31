//! Where the counts are kept: `khalid-db` keeps them in SQLite as visitor
//! data.

use std::error::Error;
use std::future::Future;
use std::pin::Pin;

use chrono::NaiveDate;

use crate::{Crawls, Tally, Views};

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

/// The counts could not be read or written.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AnalyticsError {
    /// The storage failed, with what it reported.
    #[error("the site's analytics could not be read or written")]
    Storage(#[source] Box<dyn Error + Send + Sync>),
}
