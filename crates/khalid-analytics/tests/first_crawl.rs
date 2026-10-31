//! The first-crawl timer (GEO C2): how long each new post waited for each
//! checked crawler, from the kept crawled pages.

use chrono::{DateTime, Duration, TimeZone, Utc};
use khalid_analytics::{CrawledPage, Crawler, LivePage, first_crawls};

#[cfg(test)]
fn at(day: u32, hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, day, hour, 0, 0).unwrap()
}

#[cfg(test)]
fn crawled(path: &str, name: &str, first: DateTime<Utc>, last: DateTime<Utc>) -> CrawledPage {
    CrawledPage {
        path: path.to_owned(),
        crawler: Crawler::named(name).unwrap(),
        first_at: first,
        last_at: last,
    }
}

#[cfg(test)]
fn live(path: &str, published_at: DateTime<Utc>) -> LivePage {
    LivePage {
        path: path.to_owned(),
        published_at,
    }
}

#[test]
fn each_new_post_shows_how_long_each_crawler_took_newest_post_first() {
    let posts = [
        live("/blog/old/", at(1, 0)),
        live("/blog/geo/", at(3, 9)),
        live("/blog/quiet/", at(4, 9)),
    ];
    let pages = [
        crawled("/blog/old/", "Googlebot", at(2, 0), at(5, 0)),
        crawled("/blog/geo/", "Bingbot", at(4, 9), at(4, 9)),
        crawled("/blog/geo/", "Googlebot", at(3, 15), at(6, 0)),
    ];
    let report = first_crawls(&posts, &pages);
    let paths: Vec<&str> = report.iter().map(|post| post.path.as_str()).collect();
    // The log saw its first crawl on day 2, so the day 1 post is left out.
    assert_eq!(paths, ["/blog/quiet/", "/blog/geo/"]);
    assert_eq!(report[0].crawlers, []);
    let geo: Vec<(&str, Duration)> = report[1]
        .crawlers
        .iter()
        .map(|fetch| (fetch.crawler.name(), fetch.after))
        .collect();
    assert_eq!(
        geo,
        [
            ("Googlebot", Duration::hours(6)),
            ("Bingbot", Duration::hours(24))
        ]
    );
}

#[test]
fn a_fetch_before_the_post_went_live_waited_nothing() {
    // The address answered before, under an earlier post at the same slug.
    let posts = [live("/blog/geo/", at(3, 9))];
    let pages = [
        crawled("/blog/", "Bingbot", at(1, 0), at(1, 0)),
        crawled("/blog/geo/", "Googlebot", at(3, 8), at(5, 0)),
    ];
    let report = first_crawls(&posts, &pages);
    assert_eq!(report[0].crawlers[0].after, Duration::zero());
}

#[test]
fn no_crawl_seen_yet_means_no_first_crawl_report() {
    assert_eq!(first_crawls(&[live("/blog/geo/", at(3, 9))], &[]), []);
}
