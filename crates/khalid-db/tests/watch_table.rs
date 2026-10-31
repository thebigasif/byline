//! The speed and uptime watch's looks on SQLite (C6): kept per site and
//! page, read back oldest first, and dropped when old.

use std::path::PathBuf;

use chrono::{DateTime, Duration, TimeZone, Utc};
use khalid_alerts::{Answer, Probe};
use khalid_core::Slug;
use khalid_db::{Database, WatchTable};

const HOME: &str = "https://www.example.com/";

#[cfg(test)]
fn at(minute: i64) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 3, 9, 0, 0).unwrap() + Duration::minutes(minute)
}

#[cfg(test)]
fn probe(minute: i64, answer: Answer, millis: u32) -> Probe {
    Probe {
        at: at(minute),
        answer,
        millis,
    }
}

/// A migrated database in a fresh folder, with the tables of two sites.
#[cfg(test)]
async fn tables() -> (tempfile::TempDir, WatchTable, WatchTable) {
    let dir = tempfile::tempdir().unwrap();
    let mut path = PathBuf::from(dir.path());
    path.push("site.db");
    let db = Database::open(&path).await.unwrap();
    db.migrate().await.unwrap();
    let main = db.site(&"main".parse::<Slug>().unwrap()).await.unwrap();
    let sister = db.site(&"sister".parse::<Slug>().unwrap()).await.unwrap();
    (
        dir,
        WatchTable::new(db.clone(), main),
        WatchTable::new(db, sister),
    )
}

#[tokio::test]
async fn looks_come_back_oldest_first_for_their_site_and_page_alone() {
    let (_dir, main, sister) = tables().await;
    let looks = [
        probe(10, Answer::Status(200), 120),
        probe(0, Answer::Unreached, 10_000),
        probe(5, Answer::Status(503), 40),
    ];
    for look in &looks {
        main.record(HOME, look).await.unwrap();
    }
    main.record("https://www.example.com/other/", &looks[0])
        .await
        .unwrap();
    let read = main.since(HOME, at(0)).await.unwrap();
    assert_eq!(read, [looks[1], looks[2], looks[0]]);
    assert_eq!(main.since(HOME, at(1)).await.unwrap(), [looks[2], looks[0]]);
    assert_eq!(sister.since(HOME, at(0)).await.unwrap(), []);
}

#[tokio::test]
async fn old_looks_are_dropped_and_every_page_is_listed() {
    let (_dir, main, _) = tables().await;
    main.record(HOME, &probe(0, Answer::Status(200), 90))
        .await
        .unwrap();
    main.record(
        "https://www.example.com/b/",
        &probe(9, Answer::Status(200), 90),
    )
    .await
    .unwrap();
    assert_eq!(
        main.pages().await.unwrap(),
        [HOME, "https://www.example.com/b/"]
    );
    main.forget_before(at(5)).await.unwrap();
    assert_eq!(main.since(HOME, at(-60)).await.unwrap(), []);
    assert_eq!(main.pages().await.unwrap(), ["https://www.example.com/b/"]);
}
