//! Staff accounts on SQLite keep the contract `khalid-auth` tests on its
//! in-memory store, and each site sees only its own accounts.

use std::path::PathBuf;

use chrono::{DateTime, TimeZone, Utc};
use khalid_api::keys::Role;
use khalid_auth::{Account, AccountStore, CreateAccountError, Email, PasswordHash};
use khalid_core::Slug;
use khalid_db::{AccountTable, Database};

#[cfg(test)]
fn at() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap()
}

#[cfg(test)]
fn email(text: &str) -> Email {
    text.parse().unwrap()
}

/// A migrated database with the sites `demo` and `other`, the accounts of
/// each, and Asif as the owner of `demo`.
#[cfg(test)]
async fn opened() -> (tempfile::TempDir, AccountTable, AccountTable, Account) {
    let dir = tempfile::tempdir().unwrap();
    let mut path = PathBuf::from(dir.path());
    path.push("site.db");
    let db = Database::open(&path).await.unwrap();
    db.migrate().await.unwrap();
    let demo = db.site(&Slug::try_new("demo").unwrap()).await.unwrap();
    let other = db.site(&Slug::try_new("other").unwrap()).await.unwrap();
    let demo = AccountTable::new(db.clone(), demo);
    let other = AccountTable::new(db, other);
    let owner = demo
        .create(&email("asif@example.com"), "Asif", Role::Owner, at())
        .await
        .unwrap();
    (dir, demo, other, owner)
}

#[tokio::test]
async fn an_account_reads_back_by_email_and_by_row() {
    let (_dir, demo, _, owner) = opened().await;
    let found = demo
        .find_by_email(&email("ASIF@example.com"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.account, owner);
    assert_eq!(found.password, None);
    assert_eq!(demo.find(owner.id()).await.unwrap(), Some(owner));
}

#[tokio::test]
async fn an_email_holds_one_account_and_a_site_one_owner() {
    let (_dir, demo, _, _) = opened().await;
    let again = demo
        .create(&email("asif@example.com"), "Asif", Role::Writer, at())
        .await;
    assert!(
        matches!(again, Err(CreateAccountError::EmailTaken)),
        "{again:?}"
    );
    let second = demo
        .create(&email("mia@example.com"), "Mia", Role::Owner, at())
        .await;
    assert!(
        matches!(second, Err(CreateAccountError::OwnerTaken)),
        "{second:?}"
    );
}

#[tokio::test]
async fn each_site_sees_only_its_own_accounts() {
    let (_dir, demo, other, owner) = opened().await;
    assert_eq!(other.find(owner.id()).await.unwrap(), None);
    assert_eq!(other.list().await.unwrap(), []);
    let theirs = other
        .create(&email("asif@example.com"), "Asif", Role::Owner, at())
        .await;
    assert!(
        theirs.is_ok(),
        "the same email and an owner on another site: {theirs:?}"
    );
    assert!(
        !other.remove(owner.id()).await.unwrap(),
        "cannot remove another site's account"
    );
    assert_eq!(demo.list().await.unwrap(), [owner]);
}

#[tokio::test]
async fn roles_change_but_never_to_or_from_owner_except_by_transfer() {
    let (_dir, demo, _, owner) = opened().await;
    let mia = demo
        .create(&email("mia@example.com"), "Mia", Role::Writer, at())
        .await
        .unwrap();
    assert!(demo.set_role(mia.id(), Role::Admin).await.unwrap());
    assert!(!demo.set_role(mia.id(), Role::Owner).await.unwrap());
    assert!(!demo.set_role(owner.id(), Role::Editor).await.unwrap());
    assert!(
        !demo.transfer(mia.id(), owner.id()).await.unwrap(),
        "only the owner hands over"
    );
    assert!(!demo.transfer(owner.id(), owner.id()).await.unwrap());
    assert!(demo.transfer(owner.id(), mia.id()).await.unwrap());
    let accounts = demo.list().await.unwrap();
    let roles: Vec<Role> = accounts.iter().map(Account::role).collect();
    assert_eq!(roles, [Role::Admin, Role::Owner], "Asif, then Mia");
}

#[tokio::test]
async fn a_password_is_stored_as_its_hash_and_removal_cascades() {
    let (_dir, demo, _, owner) = opened().await;
    let hash =
        PasswordHash::from_phc("$argon2id$v=19$m=8,t=1,p=1$c2FsdHNhbHQ$aGFzaA".to_owned()).unwrap();
    assert!(demo.set_password(owner.id(), &hash).await.unwrap());
    let found = demo.find_by_email(owner.email()).await.unwrap().unwrap();
    assert_eq!(found.password, Some(hash));
    assert!(demo.remove(owner.id()).await.unwrap());
    assert!(!demo.remove(owner.id()).await.unwrap());
    assert_eq!(demo.find(owner.id()).await.unwrap(), None);
}
