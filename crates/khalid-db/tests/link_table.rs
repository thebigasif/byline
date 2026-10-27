//! Sign-in links on SQLite keep the contract `khalid-auth` tests on its
//! in-memory store, and end with their account.

use std::path::PathBuf;

use chrono::{DateTime, TimeDelta, TimeZone, Utc};
use khalid_api::keys::Role;
use khalid_auth::{Account, AccountStore, LINK_LIFETIME, LinkStore, LinkToken};
use khalid_core::Slug;
use khalid_db::{AccountTable, Database, LinkTable};

#[cfg(test)]
fn start() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap()
}

/// A migrated database with the sites `demo` and `other`, the links of
/// each, the accounts of `demo`, and Mia on `demo`.
#[cfg(test)]
async fn opened() -> (
    tempfile::TempDir,
    LinkTable,
    LinkTable,
    AccountTable,
    Account,
) {
    let dir = tempfile::tempdir().unwrap();
    let mut path = PathBuf::from(dir.path());
    path.push("site.db");
    let db = Database::open(&path).await.unwrap();
    db.migrate().await.unwrap();
    let demo = db.site(&Slug::try_new("demo").unwrap()).await.unwrap();
    let other = db.site(&Slug::try_new("other").unwrap()).await.unwrap();
    let accounts = AccountTable::new(db.clone(), demo);
    let email = "mia@example.com".parse().unwrap();
    let mia = accounts
        .create(&email, "Mia", Role::Writer, start())
        .await
        .unwrap();
    let links = LinkTable::new(db.clone(), demo);
    (dir, links, LinkTable::new(db, other), accounts, mia)
}

#[tokio::test]
async fn a_link_is_used_once_and_only_before_it_expires() {
    let (_dir, links, other, _, mia) = opened().await;
    let (fresh, stale, unknown) = (
        LinkToken::generate().unwrap(),
        LinkToken::generate().unwrap(),
        LinkToken::generate().unwrap(),
    );
    links.create(fresh.hash(), mia.id(), start()).await.unwrap();
    links
        .create(stale.hash(), mia.id(), start() - LINK_LIFETIME)
        .await
        .unwrap();
    let later = start() + TimeDelta::minutes(1);
    assert_eq!(
        other.redeem(fresh.hash(), later).await.unwrap(),
        None,
        "another site's link"
    );
    assert_eq!(
        links.redeem(fresh.hash(), later).await.unwrap(),
        Some(mia.id())
    );
    assert_eq!(
        links.redeem(fresh.hash(), later).await.unwrap(),
        None,
        "used once"
    );
    assert_eq!(
        links.redeem(stale.hash(), start()).await.unwrap(),
        None,
        "expired"
    );
    assert_eq!(links.redeem(unknown.hash(), later).await.unwrap(), None);
}

#[tokio::test]
async fn removing_an_account_removes_its_links() {
    let (_dir, links, _, accounts, mia) = opened().await;
    let token = LinkToken::generate().unwrap();
    links.create(token.hash(), mia.id(), start()).await.unwrap();
    accounts.remove(mia.id()).await.unwrap();
    assert_eq!(links.redeem(token.hash(), start()).await.unwrap(), None);
}
