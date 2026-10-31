//! The analytics on SQLite: tallies add up and last, per site, and the
//! crawl log keeps its daily limit and its 90 days (KH-GRO-4).

use std::path::PathBuf;

use chrono::{DateTime, Days, NaiveDate, TimeZone, Utc};
use khalid_analytics::{
    AnalyticsStore, CRAWL_DAYS_KEPT, Crawler, CrawlerKind, MAX_CRAWL_ROWS_PER_DAY, ReferrerGroup,
    Tally,
};
use khalid_core::Slug;
use khalid_db::{AnalyticsTable, Database};

const POST: &str = "/blog/internal-links/";

#[cfg(test)]
fn at(day: u32, hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, day, hour, 0, 0).unwrap()
}

#[cfg(test)]
fn bot(user_agent: &str) -> Crawler {
    Crawler::recognise(Some(user_agent)).unwrap()
}

/// The analytics of site `handle` in a database in `dir`.
#[cfg(test)]
async fn table(dir: &tempfile::TempDir, handle: &str) -> (Database, AnalyticsTable) {
    let mut path = PathBuf::from(dir.path());
    path.push("site.db");
    let db = Database::open(&path).await.unwrap();
    db.migrate().await.unwrap();
    let site = db.site(&Slug::try_new(handle).unwrap()).await.unwrap();
    (db.clone(), AnalyticsTable::new(db, site))
}

#[tokio::test]
async fn tallies_add_up_and_outlast_closing_the_database() {
    let dir = tempfile::tempdir().unwrap();
    let (db, table) = table(&dir, "main").await;
    let gptbot = bot("GPTBot/1.2");
    for (hour, status) in [(9, 404), (11, 200)] {
        let mut tally = Tally::default();
        tally.view(at(3, hour), POST, ReferrerGroup::Ai);
        tally.view(at(3, hour), POST, ReferrerGroup::Direct);
        tally.crawl(at(3, hour), POST, gptbot, status);
        table.save(&tally).await.unwrap();
    }
    // A late write of an earlier request keeps the latest status.
    let mut late = Tally::default();
    late.crawl(at(3, 10), POST, gptbot, 500);
    table.save(&late).await.unwrap();
    db.close().await;

    let (_db, table) = self::table(&dir, "main").await;
    let day = at(3, 0).date_naive();
    let views: Vec<_> = table
        .views(day, day)
        .await
        .unwrap()
        .into_iter()
        .map(|row| (row.group, row.views))
        .collect();
    assert_eq!(views, [(ReferrerGroup::Direct, 2), (ReferrerGroup::Ai, 2)]);
    let crawls = table.crawls(day, day).await.unwrap();
    assert_eq!(crawls.len(), 1);
    let row = &crawls[0];
    assert_eq!(
        (row.crawler.as_str(), row.kind, row.hits),
        ("GPTBot", CrawlerKind::Ai, 3)
    );
    assert_eq!((row.last_seen, row.last_status), (at(3, 11), 200));
    // Another site in the same file sees none of it.
    let (_db, other) = self::table(&dir, "docs").await;
    assert_eq!(other.views(day, day).await.unwrap(), []);
    assert_eq!(other.crawls(day, day).await.unwrap(), []);
}

#[tokio::test]
async fn a_full_day_of_crawls_still_counts_its_rows_and_drops_new_ones() {
    let dir = tempfile::tempdir().unwrap();
    let (_db, table) = table(&dir, "main").await;
    let googlebot = bot("Googlebot/2.1");
    let mut tally = Tally::default();
    for page in 0..MAX_CRAWL_ROWS_PER_DAY {
        tally.crawl(at(3, 9), &format!("/blog/{page}/"), googlebot, 404);
    }
    table.save(&tally).await.unwrap();
    let mut more = Tally::default();
    more.crawl(at(3, 10), "/blog/one-too-many/", googlebot, 404);
    more.crawl(at(3, 10), "/blog/0/", googlebot, 404);
    table.save(&more).await.unwrap();
    let day = at(3, 0).date_naive();
    let crawls = table.crawls(day, day).await.unwrap();
    assert_eq!(crawls.len(), MAX_CRAWL_ROWS_PER_DAY);
    assert!(crawls.iter().all(|row| row.path != "/blog/one-too-many/"));
    assert!(
        crawls
            .iter()
            .any(|row| row.path == "/blog/0/" && row.hits == 2)
    );
}

#[tokio::test]
async fn crawls_older_than_the_kept_days_are_dropped_and_views_kept() {
    let dir = tempfile::tempdir().unwrap();
    let (_db, table) = table(&dir, "main").await;
    let old = at(3, 9);
    let mut tally = Tally::default();
    tally.view(old, POST, ReferrerGroup::Search);
    tally.crawl(old, POST, bot("bingbot/2.0"), 200);
    table.save(&tally).await.unwrap();
    let later = old + Days::new(u64::from(CRAWL_DAYS_KEPT) + 1);
    let mut tally = Tally::default();
    tally.crawl(later, POST, bot("bingbot/2.0"), 200);
    table.save(&tally).await.unwrap();
    let (first, last) = (NaiveDate::MIN, NaiveDate::MAX);
    let crawls = table.crawls(first, last).await.unwrap();
    let days: Vec<_> = crawls.iter().map(|row| row.day).collect();
    assert_eq!(days, [later.date_naive()]);
    assert_eq!(table.views(first, last).await.unwrap().len(), 1);
}
