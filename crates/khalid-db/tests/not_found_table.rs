//! The 404 log on SQLite: hits are counted and kept after the database is
//! opened again, the log stops growing when full, and each site sees only
//! its own (KH-SEO-8).

use std::path::{Path, PathBuf};

use chrono::{DateTime, TimeZone, Utc};
use khalid_api::not_found_log::{MAX_MISSES, NotFoundLog};
use khalid_core::Slug;
use khalid_db::{Database, NotFoundTable};

#[cfg(test)]
fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 3, hour, 0, 0).unwrap()
}

/// The 404 log of site `handle` in the database at `path`.
#[cfg(test)]
async fn table(path: &Path, handle: &str) -> (Database, NotFoundTable) {
    let db = Database::open(path).await.unwrap();
    db.migrate().await.unwrap();
    let site = db.site(&Slug::try_new(handle).unwrap()).await.unwrap();
    (db.clone(), NotFoundTable::new(db, site))
}

#[cfg(test)]
fn file(dir: &tempfile::TempDir) -> PathBuf {
    let mut path = PathBuf::from(dir.path());
    path.push("site.db");
    path
}

#[tokio::test]
async fn hits_are_counted_most_first_and_outlast_closing_the_database() {
    let dir = tempfile::tempdir().unwrap();
    let (db, log) = table(&file(&dir), "main").await;
    log.record("/blog/internal-links/", at(9)).await.unwrap();
    log.record("/blog/wp-login.php", at(10)).await.unwrap();
    log.record("/blog/internal-links/", at(11)).await.unwrap();
    db.close().await;

    let (db, log) = table(&file(&dir), "main").await;
    let misses = log.misses(10).await.unwrap();
    let paths: Vec<&str> = misses.iter().map(|m| m.path.as_str()).collect();
    assert_eq!(paths, ["/blog/internal-links/", "/blog/wp-login.php"]);
    assert_eq!(misses[0].hits, 2);
    assert_eq!((misses[0].first_seen, misses[0].last_seen), (at(9), at(11)));
    assert_eq!(log.misses(1).await.unwrap().len(), 1);

    assert!(log.forget("/blog/internal-links/").await.unwrap());
    assert!(!log.forget("/blog/internal-links/").await.unwrap());
    assert_eq!(log.misses(10).await.unwrap().len(), 1);
    db.close().await;
}

#[tokio::test]
async fn a_full_log_counts_known_addresses_and_drops_new_ones() {
    let dir = tempfile::tempdir().unwrap();
    let (db, log) = table(&file(&dir), "main").await;
    for n in 0..MAX_MISSES {
        log.record(&format!("/blog/probe-{n}/"), at(9))
            .await
            .unwrap();
    }
    log.record("/blog/one-more/", at(10)).await.unwrap();
    log.record("/blog/probe-7/", at(10)).await.unwrap();
    let misses = log.misses(u32::MAX).await.unwrap();
    assert_eq!(misses.len(), MAX_MISSES);
    assert_eq!(misses[0].path, "/blog/probe-7/");
    assert!(misses.iter().all(|m| m.path != "/blog/one-more/"));
    db.close().await;
}

#[tokio::test]
async fn each_site_sees_only_its_own_log() {
    let dir = tempfile::tempdir().unwrap();
    let (db, main) = table(&file(&dir), "main").await;
    let docs = NotFoundTable::new(
        db.clone(),
        db.site(&Slug::try_new("docs").unwrap()).await.unwrap(),
    );
    main.record("/blog/internal-links/", at(9)).await.unwrap();
    assert_eq!(docs.misses(10).await.unwrap(), []);
    assert!(!docs.forget("/blog/internal-links/").await.unwrap());
    assert_eq!(main.misses(10).await.unwrap().len(), 1);
    db.close().await;
}
