//! The visitor log on SQLite: rows read back as written, per site, old
//! rows are erased and a day's rows stop at the limit (GEO C1).

use std::net::IpAddr;
use std::path::PathBuf;

use chrono::{DateTime, TimeZone, Utc};
use khalid_analytics::{
    BotCheck, Crawler, MAX_VISITS_PER_DAY, ReferrerGroup, ShortAddress, Visit, Visitor, VisitorKey,
    VisitorLogStore,
};
use khalid_core::Slug;
use khalid_db::{Database, VisitorLogTable};

const POST: &str = "/blog/what-is-geo/";

#[cfg(test)]
fn at(day: u32, hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, day, hour, 0, 0).unwrap()
}

#[cfg(test)]
fn visit(at: DateTime<Utc>, address: &str, visitor: Visitor) -> Visit {
    let address: IpAddr = address.parse().unwrap();
    Visit {
        at,
        path: POST.to_owned(),
        status: 200,
        address: ShortAddress::of(address),
        code: VisitorKey::from_bytes([3; 32]).code(address, None),
        visitor,
    }
}

#[cfg(test)]
fn person(at: DateTime<Utc>) -> Visit {
    visit(at, "203.0.113.57", Visitor::Person(ReferrerGroup::Ai))
}

#[cfg(test)]
fn gptbot(at: DateTime<Utc>) -> Visit {
    let bot = Crawler::named("GPTBot").unwrap();
    visit(at, "20.171.206.9", Visitor::Bot(bot, BotCheck::Listed))
}

/// The visitor log of site `handle` in a database in `dir`.
#[cfg(test)]
async fn table(dir: &tempfile::TempDir, handle: &str) -> (Database, VisitorLogTable) {
    let mut path = PathBuf::from(dir.path());
    path.push("site.db");
    let db = Database::open(&path).await.unwrap();
    db.migrate().await.unwrap();
    let site = db.site(&Slug::try_new(handle).unwrap()).await.unwrap();
    (db.clone(), VisitorLogTable::new(db, site))
}

#[tokio::test]
async fn visits_read_back_as_written_and_outlast_closing_the_database() {
    let dir = tempfile::tempdir().unwrap();
    let (db, table) = table(&dir, "main").await;
    let written = vec![person(at(3, 9)), gptbot(at(3, 10))];
    table.save(&written, at(1, 0)).await.unwrap();
    db.close().await;

    let (_db, table) = self::table(&dir, "main").await;
    let read = table.visits(at(3, 0), at(4, 0)).await.unwrap();
    assert_eq!(read, written);
    assert_eq!(read[0].address.to_string(), "203.0.113.0");
}

#[tokio::test]
async fn no_full_address_is_ever_stored() {
    let dir = tempfile::tempdir().unwrap();
    let (db, table) = table(&dir, "main").await;
    table.save(&[person(at(3, 9))], at(1, 0)).await.unwrap();
    let mut path = PathBuf::from(dir.path());
    path.push("site.db");
    db.close().await;
    let bytes = std::fs::read(&path).unwrap();
    let text = String::from_utf8_lossy(&bytes);
    assert!(!text.contains("203.0.113.57"));
    assert!(text.contains("203.0.113.0"));
}

#[tokio::test]
async fn each_site_reads_only_its_own_visits() {
    let dir = tempfile::tempdir().unwrap();
    let (_db, main) = table(&dir, "main").await;
    let (_db, other) = table(&dir, "other").await;
    main.save(&[person(at(3, 9))], at(1, 0)).await.unwrap();
    assert_eq!(other.visits(at(1, 0), at(9, 0)).await.unwrap(), vec![]);
}

#[tokio::test]
async fn the_widest_read_finds_every_visit() {
    let dir = tempfile::tempdir().unwrap();
    let (_db, table) = table(&dir, "main").await;
    table.save(&[person(at(3, 9))], at(1, 0)).await.unwrap();
    let all = table.visits(DateTime::<Utc>::MIN_UTC, DateTime::<Utc>::MAX_UTC);
    assert_eq!(all.await.unwrap(), vec![person(at(3, 9))]);
}

#[tokio::test]
async fn a_save_erases_the_visits_before_the_kept_days() {
    let dir = tempfile::tempdir().unwrap();
    let (_db, table) = table(&dir, "main").await;
    table
        .save(&[person(at(1, 9)), gptbot(at(5, 9))], at(1, 0))
        .await
        .unwrap();
    // Nothing new, and the first day falls out.
    table.save(&[], at(2, 0)).await.unwrap();
    let left = table.visits(at(1, 0), at(9, 0)).await.unwrap();
    assert_eq!(left, vec![gptbot(at(5, 9))]);
}

#[tokio::test]
async fn a_day_stops_at_the_limit_and_the_next_day_starts_again() {
    let dir = tempfile::tempdir().unwrap();
    let (_db, table) = table(&dir, "main").await;
    let full: Vec<Visit> = (0..MAX_VISITS_PER_DAY).map(|_| person(at(3, 9))).collect();
    table.save(&full, at(1, 0)).await.unwrap();
    table
        .save(&[gptbot(at(3, 23)), gptbot(at(4, 0))], at(1, 0))
        .await
        .unwrap();
    let third = table.visits(at(3, 0), at(3, 23)).await.unwrap();
    assert_eq!(third.len(), MAX_VISITS_PER_DAY);
    assert!(
        third
            .iter()
            .all(|row| row.visitor != gptbot(at(3, 23)).visitor)
    );
    let fourth = table.visits(at(4, 0), at(4, 23)).await.unwrap();
    assert_eq!(fourth, vec![gptbot(at(4, 0))]);
}

#[tokio::test]
async fn a_listed_crawl_outlasts_its_visits_and_keeps_its_first_and_last_fetch() {
    let dir = tempfile::tempdir().unwrap();
    let (_db, main) = table(&dir, "main").await;
    let (_db, other) = table(&dir, "other").await;
    main.save(&[gptbot(at(3, 9)), person(at(3, 10))], at(1, 0))
        .await
        .unwrap();
    // A later save erases the visits, and widens the kept fetches.
    main.save(&[gptbot(at(9, 9)), gptbot(at(2, 9))], at(8, 0))
        .await
        .unwrap();
    let crawled = main.crawled().await.unwrap();
    assert_eq!(crawled.len(), 1, "{crawled:?}");
    assert_eq!(crawled[0].path, POST);
    assert_eq!(crawled[0].crawler.name(), "GPTBot");
    let kept = (crawled[0].first_at, crawled[0].last_at);
    assert_eq!(kept, (at(2, 9), at(9, 9)));
    assert_eq!(other.crawled().await.unwrap(), vec![]);
}
