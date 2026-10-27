//! The redirect routes' desk: the page addresses a missing address may be
//! sent to, and the one check-and-store the Admin API and the command line
//! share (KH-SEO-8).

use std::sync::Arc;

use chrono::{TimeZone, Utc};
use khalid_api::not_found_log::MemoryNotFoundLog;
use khalid_api::redirect_desk::{PageAddresses, RedirectDesk, StaticPages, StoreRefusal};
use khalid_api::redirect_store::{MemoryRedirectStore, Origin};
use khalid_core::FixedClock;
use khalid_seo::{RedirectEntry, SeoError, UrlPath, read_redirects_csv};

#[tokio::test]
async fn static_pages_list_their_addresses_in_order() {
    let pages = StaticPages::new(&["/blog/", "/blog/robots-txt/"]).unwrap();
    let listed = pages.addresses().await.unwrap();
    let listed: Vec<&str> = listed.iter().map(UrlPath::as_str).collect();
    assert_eq!(listed, ["/blog/", "/blog/robots-txt/"]);
    assert!(StaticPages::new(&["blog/no-slash/"]).is_err());
}

#[cfg(test)]
fn entries(rows: &str) -> Vec<RedirectEntry> {
    read_redirects_csv(&format!("from,to,status\n{rows}")).unwrap()
}

#[tokio::test]
async fn the_desk_stores_checked_redirects_and_refuses_the_rest_whole() {
    let at = Utc.with_ymd_and_hms(2026, 11, 3, 9, 0, 0).unwrap();
    let desk = RedirectDesk::new(
        Arc::new(MemoryRedirectStore::default()),
        Arc::new(MemoryNotFoundLog::default()),
        Arc::new(StaticPages::default()),
        Arc::new(FixedClock::new(at)),
    );
    let stored = desk
        .store_checked(entries("/blog/a/,/blog/b/,301\n"), Origin::Import)
        .await;
    assert_eq!(stored.unwrap()[0].created_at, at);

    let looped = desk.store_checked(
        entries("/blog/c/,/blog/d/,301\n/blog/b/,/blog/a/,301\n"),
        Origin::Import,
    );
    let refused = looped.await.unwrap_err();
    assert!(
        matches!(refused, StoreRefusal::Table(SeoError::RedirectLoop { .. })),
        "{refused:?}"
    );
    let taken = desk
        .store_checked(entries("/blog/a/,/blog/e/,302\n"), Origin::Manual)
        .await;
    let refused = taken.unwrap_err();
    assert!(
        matches!(
            refused,
            StoreRefusal::Table(SeoError::RedirectDuplicate { .. })
        ),
        "{refused:?}"
    );

    let table = desk.table().await.unwrap();
    let hit = table
        .resolve(&UrlPath::try_new("/blog/a/").unwrap())
        .unwrap();
    assert_eq!(
        hit.location().map(ToString::to_string).as_deref(),
        Some("/blog/b/")
    );
    assert_eq!(table.entries().len(), 1, "nothing from the refused batches");
}
