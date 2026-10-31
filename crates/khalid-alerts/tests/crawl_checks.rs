//! The crawl alerts (GEO C4): a search crawler that stopped coming, and
//! server errors crawlers got.

use chrono::{DateTime, TimeZone, Utc};
use khalid_alerts::{CrawlerSeen, Kind, ServerError, crawl_stopped, crawler_server_errors};

#[cfg(test)]
fn at(day: u32, hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, day, hour, 0, 0).unwrap()
}

#[cfg(test)]
fn seen(crawler: &str, last_at: DateTime<Utc>) -> CrawlerSeen {
    CrawlerSeen {
        crawler: crawler.to_owned(),
        last_at,
    }
}

#[test]
fn a_crawler_unseen_for_a_week_is_raised_once_per_last_visit() {
    let last = [seen("Googlebot", at(2, 9)), seen("Bingbot", at(8, 9))];
    let found = crawl_stopped(&last, at(9, 10));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].key, "crawl-stopped:Googlebot:2026-11-02");
    assert_eq!(
        found[0].line,
        "Googlebot last fetched a page on 2026-11-02, 7 days ago."
    );
    assert!(Kind::CrawlStopped.owns(&found[0].key));
    assert!(
        Kind::CrawlStopped.clears(),
        "it clears once the crawler is back"
    );
}

#[test]
fn a_crawler_seen_within_the_week_is_not_raised() {
    assert_eq!(
        crawl_stopped(&[seen("Googlebot", at(3, 11))], at(9, 10)),
        []
    );
}

#[test]
fn server_errors_are_raised_per_address_and_hour_and_client_errors_are_not() {
    let errors = [
        ServerError {
            path: "/blog/broken/".to_owned(),
            status: 503,
            crawlers: vec!["Googlebot".to_owned(), "GPTBot".to_owned()],
            hits: 4,
        },
        ServerError {
            path: "/blog/gone/".to_owned(),
            status: 404,
            crawlers: vec!["Googlebot".to_owned()],
            hits: 9,
        },
    ];
    let found = crawler_server_errors(&errors, at(9, 10));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].key, "crawler-errors:2026-11-09T10:/blog/broken/");
    assert_eq!(
        found[0].line,
        "/blog/broken/: 503 to Googlebot, GPTBot, 4 times"
    );
    assert!(!Kind::CrawlerErrors.clears());
}
