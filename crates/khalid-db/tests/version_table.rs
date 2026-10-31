//! Earlier post versions on SQLite: kept per site and per post, the newest
//! first, only the newest 100, and kept by a rebuild (KH-EDT-2).

use std::path::PathBuf;

use chrono::{DateTime, Duration, TimeZone, Utc};
use khalid_api::version_store::{MAX_VERSIONS, PostVersion, VersionStore};
use khalid_core::Slug;
use khalid_db::{Database, VersionTable};

const ID: &str = "en.choosing-anchor-text";

#[cfg(test)]
fn at() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 20, 8, 30, 0).unwrap()
}

/// A migrated database in a fresh folder, with the tables of two sites.
#[cfg(test)]
async fn tables() -> (tempfile::TempDir, VersionTable, VersionTable) {
    let dir = tempfile::tempdir().unwrap();
    let mut path = PathBuf::from(dir.path());
    path.push("site.db");
    let db = Database::open(&path).await.unwrap();
    db.migrate().await.unwrap();
    let main = db.site(&"main".parse::<Slug>().unwrap()).await.unwrap();
    let sister = db.site(&"sister".parse::<Slug>().unwrap()).await.unwrap();
    (
        dir,
        VersionTable::new(db.clone(), main),
        VersionTable::new(db, sister),
    )
}

#[tokio::test]
async fn versions_come_back_the_newest_first_per_site_and_post() {
    let (_dir, main, sister) = tables().await;
    let first = PostVersion::new((at(), Some("asif".to_owned())), "Choosing", "Old.");
    let later = (at() + Duration::hours(2), None);
    let second = PostVersion::new(later, "Choosing anchor text", "Older text, «quoted».");
    main.keep(ID, first.clone()).await.unwrap();
    main.keep(ID, second.clone()).await.unwrap();
    main.keep("en.other", first.clone()).await.unwrap();
    assert_eq!(main.list(ID).await.unwrap(), [second, first]);
    assert_eq!(sister.list(ID).await.unwrap(), []);
    assert_eq!(main.list("en.nothing").await.unwrap(), []);
}

#[tokio::test]
async fn only_the_newest_versions_of_a_post_are_kept() {
    let (_dir, main, _) = tables().await;
    for n in 0..=MAX_VERSIONS {
        let version = PostVersion::new((at(), None), "Title", format!("Text {n}."));
        main.keep(ID, version).await.unwrap();
    }
    main.keep("en.other", PostVersion::new((at(), None), "T", "Kept."))
        .await
        .unwrap();
    let kept = main.list(ID).await.unwrap();
    assert_eq!(kept.len(), MAX_VERSIONS);
    assert_eq!(kept[0].markdown(), format!("Text {MAX_VERSIONS}."));
    assert_eq!(kept[MAX_VERSIONS - 1].markdown(), "Text 1.");
    assert_eq!(main.list("en.other").await.unwrap().len(), 1);
}
