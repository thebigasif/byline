//! Stored redirects, kept by a `RedirectStore` (KH-SEO-8). Every store
//! keeps this contract; `khalid-db` runs the same cases on SQLite.

use chrono::{DateTime, TimeZone, Utc};
use khalid_api::redirect_store::{MemoryRedirectStore, Origin, RedirectStore, RedirectStoreError};
use khalid_seo::{RedirectEntry, read_redirects_csv};

#[cfg(test)]
fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 3, hour, 0, 0).unwrap()
}

#[cfg(test)]
fn entries(rows: &str) -> Vec<RedirectEntry> {
    read_redirects_csv(&format!("from,to,status,match\n{rows}")).unwrap()
}

#[tokio::test]
async fn added_redirects_keep_their_order_origin_and_time() {
    let store = MemoryRedirectStore::default();
    let first = entries("/blog/old-links/,/blog/why-internal-links-matter/,301\n");
    let added = store
        .add(first.clone(), Origin::Manual, at(9))
        .await
        .unwrap();
    assert_eq!(added.len(), 1);
    let more = entries("/blog/retired-tool/,,410\n/blog/\\d{4}/(.+),/blog/$1,301,regex\n");
    store
        .add(more.clone(), Origin::Import, at(10))
        .await
        .unwrap();

    let listed = store.list().await.unwrap();
    let kept: Vec<_> = listed.iter().map(|r| r.entry.clone()).collect();
    assert_eq!(kept, [first, more].concat());
    assert_eq!(listed[0], added[0]);
    assert_eq!(listed[0].origin, Origin::Manual);
    assert_eq!(listed[1].origin, Origin::Import);
    assert_eq!(listed[1].created_at, at(10));
    assert!(listed[0].id < listed[1].id && listed[1].id < listed[2].id);
}

#[tokio::test]
async fn a_batch_with_a_taken_address_adds_nothing() {
    let store = MemoryRedirectStore::default();
    store
        .add(
            entries("/blog/sale/,/blog/offers/,302\n"),
            Origin::Manual,
            at(9),
        )
        .await
        .unwrap();
    let batch = entries("/blog/new/,/blog/offers/,301\n/blog/sale/,/blog/offers-2026/,301\n");
    let refused = store.add(batch, Origin::Import, at(10)).await.unwrap_err();
    assert!(
        matches!(&refused, RedirectStoreError::Taken { from } if from == "/blog/sale/"),
        "{refused:?}"
    );
    let twice = entries("/blog/a/,/blog/b/,301\n/blog/a/,/blog/c/,301\n");
    let refused = store.add(twice, Origin::Import, at(10)).await.unwrap_err();
    assert!(
        matches!(refused, RedirectStoreError::Taken { .. }),
        "{refused:?}"
    );
    assert_eq!(store.list().await.unwrap().len(), 1);
}

#[tokio::test]
async fn a_redirect_is_removed_by_its_identity() {
    let store = MemoryRedirectStore::default();
    let added = store
        .add(
            entries("/blog/sale/,/blog/offers/,302\n"),
            Origin::Manual,
            at(9),
        )
        .await
        .unwrap();
    assert!(store.remove(added[0].id).await.unwrap());
    assert!(!store.remove(added[0].id).await.unwrap());
    assert_eq!(store.list().await.unwrap(), []);
}

#[test]
fn each_origin_reads_back_from_its_name() {
    for origin in [Origin::Manual, Origin::Import, Origin::SlugChange] {
        assert_eq!(Origin::from_name(origin.as_str()), Some(origin));
    }
    assert_eq!(Origin::from_name("plugin"), None);
}
