//! The tally `khalid serve` keeps between writes.

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use khalid_analytics::{Crawler, MAX_ROWS, ReferrerGroup, Tally};

#[cfg(test)]
fn at(day: u32, hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, day, hour, 0, 0).unwrap()
}

#[cfg(test)]
fn date(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, day).unwrap()
}

#[cfg(test)]
fn bot(user_agent: &str) -> Crawler {
    Crawler::recognise(Some(user_agent)).unwrap()
}

#[test]
fn views_are_counted_per_page_day_and_group() {
    let mut tally = Tally::default();
    assert!(tally.is_empty());
    tally.view(at(3, 9), "/blog/internal-links/", ReferrerGroup::Ai);
    tally.view(at(3, 23), "/blog/internal-links/", ReferrerGroup::Ai);
    tally.view(at(4, 0), "/blog/internal-links/", ReferrerGroup::Ai);
    tally.view(at(3, 9), "/blog/internal-links/", ReferrerGroup::Search);
    let rows: Vec<_> = tally
        .views()
        .into_iter()
        .map(|row| (row.day, row.group, row.views))
        .collect();
    assert_eq!(
        rows,
        [
            (date(3), ReferrerGroup::Search, 1),
            (date(3), ReferrerGroup::Ai, 2),
            (date(4), ReferrerGroup::Ai, 1),
        ]
    );
}

#[test]
fn a_crawl_keeps_its_latest_time_and_status() {
    let mut tally = Tally::default();
    let googlebot = bot("Googlebot/2.1");
    tally.crawl(at(3, 9), "/blog/old/", googlebot, 404);
    tally.crawl(at(3, 11), "/blog/old/", googlebot, 301);
    tally.crawl(at(3, 10), "/blog/old/", googlebot, 500);
    let crawls = tally.crawls();
    assert_eq!(crawls.len(), 1);
    let row = &crawls[0];
    assert_eq!((row.crawler.as_str(), row.hits), ("Googlebot", 3));
    assert_eq!((row.last_seen, row.last_status), (at(3, 11), 301));
}

#[test]
fn a_full_tally_still_counts_its_rows_and_drops_new_ones() {
    let mut tally = Tally::default();
    for page in 0..MAX_ROWS {
        tally.view(at(3, 9), &format!("/blog/{page}/"), ReferrerGroup::Direct);
    }
    tally.view(at(3, 9), "/blog/one-too-many/", ReferrerGroup::Direct);
    tally.crawl(at(3, 9), "/blog/one-too-many/", bot("GPTBot/1.2"), 200);
    tally.view(at(3, 9), "/blog/0/", ReferrerGroup::Direct);
    let views = tally.views();
    assert_eq!(views.len(), MAX_ROWS);
    assert!(
        views
            .iter()
            .any(|row| row.path == "/blog/0/" && row.views == 2)
    );
    assert_eq!(tally.crawls(), []);
}
