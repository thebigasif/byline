//! Stored redirects on SQLite: every status and both match kinds come back
//! after the database is opened again, a batch with a taken address stores
//! nothing, and each site sees only its own (KH-SEO-8).

use std::path::{Path, PathBuf};

use chrono::{DateTime, TimeZone, Utc};
use khalid_api::redirect_store::{Origin, RedirectStore, RedirectStoreError};
use khalid_core::Slug;
use khalid_db::{Database, RedirectRows};
use khalid_seo::{RedirectEntry, read_redirects_csv};

#[cfg(test)]
fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 3, hour, 0, 0).unwrap()
}

#[cfg(test)]
fn entries(rows: &str) -> Vec<RedirectEntry> {
    read_redirects_csv(&format!("from,to,status,match\n{rows}")).unwrap()
}

/// The redirects of site `handle` in the database at `path`.
#[cfg(test)]
async fn table(path: &Path, handle: &str) -> (Database, RedirectRows) {
    let db = Database::open(path).await.unwrap();
    db.migrate().await.unwrap();
    let site = db.site(&Slug::try_new(handle).unwrap()).await.unwrap();
    (db.clone(), RedirectRows::new(db, site))
}

#[cfg(test)]
fn file(dir: &tempfile::TempDir) -> PathBuf {
    let mut path = PathBuf::from(dir.path());
    path.push("site.db");
    path
}

const EVERY_KIND: &str = "\
/blog/old-links/,/blog/why-internal-links-matter/,301
/blog/sale/,/blog/offers/,302
/blog/offers/,/blog/offers-2026/,307
/blog/retired-tool/,,410
/blog/court-order/,,451
/blog/serpdad/,https://serpdad.com/,301
/blog/\\d{4}/\\d{2}/(.+),/blog/$1,301,regex
/blog/tmp/.*,,410,regex
";

#[tokio::test]
async fn every_kind_comes_back_in_order_after_closing_the_database() {
    let dir = tempfile::tempdir().unwrap();
    let (db, rows) = table(&file(&dir), "main").await;
    let manual = entries("/blog/first/,/blog/second/,301\n");
    let added = rows
        .add(manual.clone(), Origin::Manual, at(9))
        .await
        .unwrap();
    rows.add(entries(EVERY_KIND), Origin::Import, at(10))
        .await
        .unwrap();
    db.close().await;

    let (db, rows) = table(&file(&dir), "main").await;
    let listed = rows.list().await.unwrap();
    let kept: Vec<_> = listed.iter().map(|r| r.entry.clone()).collect();
    assert_eq!(kept, [manual, entries(EVERY_KIND)].concat());
    assert_eq!(listed[0], added[0]);
    assert_eq!(
        (listed[1].origin, listed[1].created_at),
        (Origin::Import, at(10))
    );
    assert!(listed.windows(2).all(|pair| pair[0].id < pair[1].id));
    db.close().await;
}

#[tokio::test]
async fn a_batch_with_a_taken_address_stores_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let (db, rows) = table(&file(&dir), "main").await;
    rows.add(
        entries("/blog/sale/,/blog/offers/,302\n"),
        Origin::Manual,
        at(9),
    )
    .await
    .unwrap();
    let batch = entries("/blog/new/,/blog/offers/,301\n/blog/sale/,/blog/offers-2026/,301\n");
    let refused = rows.add(batch, Origin::Import, at(10)).await.unwrap_err();
    assert!(
        matches!(&refused, RedirectStoreError::Taken { from } if from == "/blog/sale/"),
        "{refused:?}"
    );
    let twice = entries("/blog/a/,/blog/b/,301\n/blog/a/,/blog/c/,301\n");
    let refused = rows.add(twice, Origin::Import, at(10)).await.unwrap_err();
    assert!(
        matches!(refused, RedirectStoreError::Taken { .. }),
        "{refused:?}"
    );
    assert_eq!(rows.list().await.unwrap().len(), 1);
    db.close().await;
}

#[tokio::test]
async fn a_redirect_is_removed_and_each_site_keeps_its_own() {
    let dir = tempfile::tempdir().unwrap();
    let (db, main) = table(&file(&dir), "main").await;
    let docs = RedirectRows::new(
        db.clone(),
        db.site(&Slug::try_new("docs").unwrap()).await.unwrap(),
    );
    let added = main
        .add(
            entries("/blog/sale/,/blog/offers/,302\n"),
            Origin::SlugChange,
            at(9),
        )
        .await
        .unwrap();
    docs.add(
        entries("/blog/sale/,/blog/elsewhere/,301\n"),
        Origin::Manual,
        at(9),
    )
    .await
    .unwrap();
    assert!(!docs.remove(added[0].id).await.unwrap());
    assert!(main.remove(added[0].id).await.unwrap());
    assert_eq!(main.list().await.unwrap(), []);
    assert_eq!(docs.list().await.unwrap().len(), 1);
    db.close().await;
}
