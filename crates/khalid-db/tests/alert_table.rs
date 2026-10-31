//! The raised alerts on SQLite: kept per kind and per site, dropped when
//! a problem clears or the key is old, and kept by a rebuild (KH-GRO-6).

use std::path::PathBuf;

use chrono::{DateTime, Duration, TimeZone, Utc};
use khalid_alerts::{Finding, Kind, Raised, fresh};
use khalid_core::Slug;
use khalid_db::{AlertTable, Database};

#[cfg(test)]
fn at() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 3, 9, 0, 0).unwrap()
}

/// A migrated database in a fresh folder, with the tables of two sites.
#[cfg(test)]
async fn tables() -> (tempfile::TempDir, AlertTable, AlertTable) {
    let dir = tempfile::tempdir().unwrap();
    let mut path = PathBuf::from(dir.path());
    path.push("site.db");
    let db = Database::open(&path).await.unwrap();
    db.migrate().await.unwrap();
    let main = db.site(&"main".parse::<Slug>().unwrap()).await.unwrap();
    let sister = db.site(&"sister".parse::<Slug>().unwrap()).await.unwrap();
    (
        dir,
        AlertTable::new(db.clone(), main),
        AlertTable::new(db, sister),
    )
}

#[tokio::test]
async fn keys_are_kept_per_kind_and_per_site() {
    let (_dir, main, sister) = tables().await;
    let keys = [Kind::StalePost.key("en.a"), Kind::Certificate.key("blog")];
    main.remember(&keys, at()).await.unwrap();
    let stale = main.raised(Kind::StalePost).await.unwrap();
    assert_eq!(stale.into_iter().collect::<Vec<_>>(), ["stale-post:en.a"]);
    assert_eq!(sister.raised(Kind::StalePost).await.unwrap().len(), 0);
    main.forget(&keys[..1]).await.unwrap();
    assert_eq!(main.raised(Kind::StalePost).await.unwrap().len(), 0);
    assert_eq!(main.raised(Kind::Certificate).await.unwrap().len(), 1);
    main.forget_before(at() + Duration::days(1)).await.unwrap();
    assert_eq!(main.raised(Kind::Certificate).await.unwrap().len(), 0);
}

#[tokio::test]
async fn a_finding_kept_in_the_table_is_not_raised_again() {
    let (_dir, main, _) = tables().await;
    let found = || {
        vec![Finding {
            key: Kind::StalePost.key("en.a"),
            line: "A".to_owned(),
        }]
    };
    let first = fresh(&main, Kind::StalePost, found(), at()).await.unwrap();
    assert!(first.is_some(), "raised the first time");
    let again = fresh(&main, Kind::StalePost, found(), at()).await.unwrap();
    assert_eq!(again, None, "kept, so not raised again");
}
