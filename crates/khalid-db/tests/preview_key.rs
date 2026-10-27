//! The site's preview key: made once, at random, and the same after the
//! database is opened again, so preview links outlast a restart; each
//! site has its own (CTO answers 2026-10-27 section 4).

use std::path::{Path, PathBuf};

use chrono::{TimeZone, Utc};
use khalid_core::{FixedClock, Slug};
use khalid_db::Database;

/// The preview key of site `handle` in the database at `path`.
#[cfg(test)]
async fn key_of(path: &Path, handle: &str) -> [u8; 32] {
    let db = Database::open(path).await.unwrap();
    db.migrate().await.unwrap();
    let site = db.site(&Slug::try_new(handle).unwrap()).await.unwrap();
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap());
    let key = *db.preview_key(site, &clock).await.unwrap().expose();
    db.close().await;
    key
}

#[tokio::test]
async fn the_preview_key_is_made_once_and_kept() {
    let dir = tempfile::tempdir().unwrap();
    let mut path = PathBuf::from(dir.path());
    path.push("site.db");
    let first = key_of(&path, "main").await;
    assert_ne!(first, [0; 32]);
    assert_eq!(key_of(&path, "main").await, first);
    assert_ne!(key_of(&path, "sister").await, first);
}
