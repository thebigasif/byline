//! Post schedules on SQLite: what is set comes back after the database is
//! opened again, an empty schedule clears, due times come oldest first,
//! and each site sees only its own (CTO answers 2026-10-27 section 4).

use std::path::{Path, PathBuf};

use chrono::{DateTime, TimeZone, Utc};
use khalid_api::schedule_store::{Due, Schedule, ScheduleStore};
use khalid_core::Slug;
use khalid_db::{Database, ScheduleTable};

#[cfg(test)]
fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 3, hour, 0, 0).unwrap()
}

/// The schedules of site `handle` in the database at `path`.
#[cfg(test)]
async fn table(path: &Path, handle: &str) -> (Database, ScheduleTable) {
    let db = Database::open(path).await.unwrap();
    db.migrate().await.unwrap();
    let site = db.site(&Slug::try_new(handle).unwrap()).await.unwrap();
    (db.clone(), ScheduleTable::new(db, site))
}

#[cfg(test)]
fn file(dir: &tempfile::TempDir) -> PathBuf {
    let mut path = PathBuf::from(dir.path());
    path.push("site.db");
    path
}

#[tokio::test]
async fn a_schedule_outlasts_closing_the_database() {
    let dir = tempfile::tempdir().unwrap();
    let schedule = Schedule::default()
        .publishing_at(at(9))
        .unpublishing_at(at(18));
    let (db, schedules) = table(&file(&dir), "main").await;
    schedules
        .set("en.choosing-anchor-text", schedule)
        .await
        .unwrap();
    db.close().await;

    let (_db, schedules) = table(&file(&dir), "main").await;
    assert_eq!(
        schedules.get("en.choosing-anchor-text").await.unwrap(),
        schedule
    );
    assert_eq!(
        schedules.get("en.no-such-post").await.unwrap(),
        Schedule::default()
    );
}

#[tokio::test]
async fn an_empty_schedule_clears_and_a_new_one_replaces() {
    let dir = tempfile::tempdir().unwrap();
    let (_db, schedules) = table(&file(&dir), "main").await;
    let id = "en.choosing-anchor-text";
    schedules
        .set(id, Schedule::default().publishing_at(at(9)))
        .await
        .unwrap();
    schedules
        .set(id, Schedule::default().publishing_at(at(11)))
        .await
        .unwrap();
    assert_eq!(schedules.get(id).await.unwrap().publish_at(), Some(at(11)));
    schedules.set(id, Schedule::default()).await.unwrap();
    assert_eq!(schedules.due(at(23)).await.unwrap(), []);
}

#[tokio::test]
async fn due_times_come_oldest_first_for_this_site_only() {
    let dir = tempfile::tempdir().unwrap();
    let (_db, main) = table(&file(&dir), "main").await;
    let (_db, sister) = table(&file(&dir), "sister").await;
    let both = Schedule::default()
        .publishing_at(at(9))
        .unpublishing_at(at(10));
    main.set("en.choosing-anchor-text", both).await.unwrap();
    main.set(
        "en.what-a-sitemap-does",
        Schedule::default().publishing_at(at(8)),
    )
    .await
    .unwrap();
    main.set("de.later", Schedule::default().publishing_at(at(20)))
        .await
        .unwrap();
    sister
        .set("en.other-site", Schedule::default().publishing_at(at(7)))
        .await
        .unwrap();
    let due = main.due(at(12)).await.unwrap();
    assert_eq!(
        due,
        [
            ("en.what-a-sitemap-does".to_owned(), Due::Publish),
            ("en.choosing-anchor-text".to_owned(), Due::Publish),
            ("en.choosing-anchor-text".to_owned(), Due::Unpublish),
        ]
    );
}
