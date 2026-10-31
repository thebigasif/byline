//! What `khalid serve` counted since its last write, held in memory so a
//! view costs a map update rather than a database write.

use std::collections::BTreeMap;

use chrono::{DateTime, NaiveDate, Utc};

use crate::{Crawler, CrawlerKind, ReferrerGroup};

/// The most rows one tally holds. Past it, rows already in the tally are
/// still counted and new ones are dropped, so a flood of made-up addresses
/// cannot grow memory without end.
pub const MAX_ROWS: usize = 10_000;

/// The views of one page on one day from one referrer group.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Views {
    /// The day, in UTC.
    pub day: NaiveDate,
    /// The page's path, such as `/blog/internal-links/`.
    pub path: String,
    /// Where the readers came from.
    pub group: ReferrerGroup,
    /// How many views.
    pub views: u64,
}

impl Views {
    /// A row of views, as a store reads it back.
    pub fn new(day: NaiveDate, path: &str, group: ReferrerGroup, views: u64) -> Self {
        Self {
            day,
            path: path.to_owned(),
            group,
            views,
        }
    }
}

/// One bot's requests for one address on one day.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Crawls {
    /// The day, in UTC.
    pub day: NaiveDate,
    /// The address asked for, such as `/blog/internal-links/`.
    pub path: String,
    /// The bot's name, such as `Googlebot`.
    pub crawler: String,
    /// What kind of bot it is.
    pub kind: CrawlerKind,
    /// How many requests.
    pub hits: u64,
    /// The latest request.
    pub last_seen: DateTime<Utc>,
    /// The status code the latest request was answered with, such as 200
    /// or 404.
    pub last_status: u16,
}

/// The count kept for one bot, address and day.
#[derive(Debug, Clone, Copy)]
struct CrawlCount {
    kind: CrawlerKind,
    hits: u64,
    last_seen: DateTime<Utc>,
    last_status: u16,
}

/// Views and crawls counted since the last write. Keys are by path first,
/// so counting a page already in the tally allocates nothing.
#[derive(Debug, Default)]
pub struct Tally {
    views: BTreeMap<String, BTreeMap<(NaiveDate, ReferrerGroup), u64>>,
    crawls: BTreeMap<String, BTreeMap<(NaiveDate, &'static str), CrawlCount>>,
    rows: usize,
}

impl Tally {
    /// Counts a reader's view of `path` at `at`, from `group`.
    pub fn view(&mut self, at: DateTime<Utc>, path: &str, group: ReferrerGroup) {
        let key = (at.date_naive(), group);
        if let Some(count) = self.views.get_mut(path).and_then(|days| days.get_mut(&key)) {
            *count = count.saturating_add(1);
        } else if self.rows < MAX_ROWS {
            self.rows += 1;
            self.views
                .entry(path.to_owned())
                .or_default()
                .insert(key, 1);
        }
    }

    /// Counts `crawler`'s request for `path` at `at`, answered with
    /// `status`.
    pub fn crawl(&mut self, at: DateTime<Utc>, path: &str, crawler: Crawler, status: u16) {
        let key = (at.date_naive(), crawler.name());
        if let Some(count) = self
            .crawls
            .get_mut(path)
            .and_then(|days| days.get_mut(&key))
        {
            count.hits = count.hits.saturating_add(1);
            if at >= count.last_seen {
                count.last_seen = at;
                count.last_status = status;
            }
        } else if self.rows < MAX_ROWS {
            self.rows += 1;
            let count = CrawlCount {
                kind: crawler.kind(),
                hits: 1,
                last_seen: at,
                last_status: status,
            };
            self.crawls
                .entry(path.to_owned())
                .or_default()
                .insert(key, count);
        }
    }

    /// Whether nothing was counted.
    pub fn is_empty(&self) -> bool {
        self.rows == 0
    }

    /// The views counted, by path, then day, then group.
    pub fn views(&self) -> Vec<Views> {
        self.views
            .iter()
            .flat_map(|(path, days)| {
                days.iter()
                    .map(|(&(day, group), &views)| Views::new(day, path, group, views))
            })
            .collect()
    }

    /// The crawls counted, by path, then day, then bot.
    pub fn crawls(&self) -> Vec<Crawls> {
        self.crawls
            .iter()
            .flat_map(|(path, days)| {
                days.iter().map(|(&(day, crawler), count)| Crawls {
                    day,
                    path: path.clone(),
                    crawler: crawler.to_owned(),
                    kind: count.kind,
                    hits: count.hits,
                    last_seen: count.last_seen,
                    last_status: count.last_status,
                })
            })
            .collect()
    }
}
