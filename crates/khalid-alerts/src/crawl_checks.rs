//! The crawl checks (GEO C4): each reads what the binary took from the
//! visitor log, and nothing else.

use chrono::{DateTime, Duration, Utc};

use crate::{Finding, Kind};

/// A search crawler unseen for this many days is raised. A week is
/// Khalid's own choice, not SEO advice: long enough that a quiet few days
/// on a small site are not raised.
pub const CRAWL_STOPPED_DAYS: i64 = 7;

/// When a crawler last fetched a page from an address on its list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrawlerSeen {
    /// Its name, such as `Googlebot`.
    pub crawler: String,
    /// Its latest visit.
    pub last_at: DateTime<Utc>,
}

/// A finding for each crawler in `seen` whose latest visit is
/// [`CRAWL_STOPPED_DAYS`] or more before `at`. The key names the day of
/// that visit, so a crawler that comes back and stops again is raised
/// again.
pub fn crawl_stopped(seen: &[CrawlerSeen], at: DateTime<Utc>) -> Vec<Finding> {
    seen.iter()
        .filter(|one| at - one.last_at >= Duration::days(CRAWL_STOPPED_DAYS))
        .map(|one| {
            let day = one.last_at.format("%Y-%m-%d");
            let days = (at - one.last_at).num_days();
            Finding {
                key: Kind::CrawlStopped.key(&format!("{}:{day}", one.crawler)),
                line: format!(
                    "{} last fetched a page on {day}, {days} days ago.",
                    one.crawler
                ),
            }
        })
        .collect()
}

/// One address that answered crawlers with one error status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerError {
    /// The address, such as `/blog/geo/`.
    pub path: String,
    /// The status it answered with.
    pub status: u16,
    /// The crawlers that got it, by name.
    pub crawlers: Vec<String>,
    /// How many times.
    pub hits: u64,
}

/// A finding for each address in `errors` that answered with a 5xx. Keys
/// name the hour of `at`, so an error that goes on is raised again each
/// hour it is seen. A 4xx is left to the 404 alert and the report.
pub fn crawler_server_errors(errors: &[ServerError], at: DateTime<Utc>) -> Vec<Finding> {
    let hour = at.format("%Y-%m-%dT%H");
    errors
        .iter()
        .filter(|error| error.status >= 500)
        .map(|error| {
            let times = match error.hits {
                1 => "once".to_owned(),
                many => format!("{many} times"),
            };
            Finding {
                key: Kind::CrawlerErrors.key(&format!("{hour}:{}", error.path)),
                line: format!(
                    "{}: {} to {}, {times}",
                    error.path,
                    error.status,
                    error.crawlers.join(", ")
                ),
            }
        })
        .collect()
}
