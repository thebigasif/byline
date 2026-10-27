//! The log of addresses that answered 404, kept by a `NotFoundLog`
//! (KH-SEO-8). `khalid-db` runs these cases on SQLite, with the full log.

use chrono::{DateTime, TimeZone, Utc};
use khalid_api::not_found_log::{MemoryNotFoundLog, NotFoundLog};

#[cfg(test)]
fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 3, hour, 0, 0).unwrap()
}

#[tokio::test]
async fn misses_are_counted_most_hit_first_and_forgotten() {
    let log = MemoryNotFoundLog::default();
    log.record("/blog/internal-links/", at(9)).await.unwrap();
    log.record("/blog/wp-login.php", at(10)).await.unwrap();
    log.record("/blog/internal-links/", at(11)).await.unwrap();

    let misses = log.misses(10).await.unwrap();
    let paths: Vec<&str> = misses.iter().map(|m| m.path.as_str()).collect();
    assert_eq!(paths, ["/blog/internal-links/", "/blog/wp-login.php"]);
    assert_eq!(misses[0].hits, 2);
    assert_eq!((misses[0].first_seen, misses[0].last_seen), (at(9), at(11)));
    assert_eq!(log.misses(1).await.unwrap().len(), 1);

    assert!(log.forget("/blog/internal-links/").await.unwrap());
    assert!(!log.forget("/blog/internal-links/").await.unwrap());
    assert_eq!(log.misses(10).await.unwrap().len(), 1);
}
