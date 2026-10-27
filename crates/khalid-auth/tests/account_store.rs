//! What every account store must do, shown on the one in memory. The
//! SQLite store in `khalid-db` runs the same cases.

use chrono::{DateTime, TimeZone, Utc};
use khalid_api::keys::Role;
use khalid_auth::{AccountStore, CreateAccountError, Email, MemoryAccounts, PasswordHash};

#[cfg(test)]
fn email(text: &str) -> Email {
    text.parse().expect("an email address")
}

#[cfg(test)]
fn at() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0)
        .single()
        .expect("a time")
}

#[cfg(test)]
async fn store_with_owner() -> (MemoryAccounts, khalid_auth::Account) {
    let store = MemoryAccounts::new();
    let owner = store
        .create(&email("asif@example.com"), "Asif", Role::Owner, at())
        .await
        .expect("the first owner");
    (store, owner)
}

#[tokio::test]
async fn an_account_is_found_by_its_email_in_any_case_and_has_no_password_yet() {
    let (store, owner) = store_with_owner().await;
    let found = store
        .find_by_email(&email("ASIF@example.com"))
        .await
        .expect("read");
    let found = found.expect("the owner");
    assert_eq!(found.account, owner);
    assert_eq!(found.password, None);
    assert_eq!(store.find(owner.id()).await.expect("read"), Some(owner));
}

#[tokio::test]
async fn an_email_holds_one_account_and_a_site_one_owner() {
    let (store, _) = store_with_owner().await;
    let again = store
        .create(&email("asif@example.com"), "Asif", Role::Writer, at())
        .await;
    assert!(
        matches!(again, Err(CreateAccountError::EmailTaken)),
        "{again:?}"
    );
    let second = store
        .create(&email("mia@example.com"), "Mia", Role::Owner, at())
        .await;
    assert!(
        matches!(second, Err(CreateAccountError::OwnerTaken)),
        "{second:?}"
    );
}

#[tokio::test]
async fn accounts_are_listed_by_email() {
    let (store, _) = store_with_owner().await;
    for (address, role) in [
        ("zoe@example.com", Role::Writer),
        ("ben@example.com", Role::Editor),
    ] {
        store
            .create(&email(address), "Staff", role, at())
            .await
            .expect("made");
    }
    let listed = store.list().await.expect("read");
    let emails: Vec<&str> = listed
        .iter()
        .map(|account| account.email().as_str())
        .collect();
    assert_eq!(
        emails,
        ["asif@example.com", "ben@example.com", "zoe@example.com"]
    );
}

#[tokio::test]
async fn a_role_change_never_makes_or_unmakes_the_owner() {
    let (store, owner) = store_with_owner().await;
    let writer = store
        .create(&email("mia@example.com"), "Mia", Role::Writer, at())
        .await
        .expect("made");
    assert!(
        store
            .set_role(writer.id(), Role::Editor)
            .await
            .expect("written")
    );
    assert!(
        !store
            .set_role(writer.id(), Role::Owner)
            .await
            .expect("written")
    );
    assert!(
        !store
            .set_role(owner.id(), Role::Admin)
            .await
            .expect("written")
    );
    let roles: Vec<Role> = store
        .list()
        .await
        .expect("read")
        .iter()
        .map(khalid_auth::Account::role)
        .collect();
    assert_eq!(roles, [Role::Owner, Role::Editor]);
}

#[tokio::test]
async fn a_transfer_swaps_the_owner_with_another_account_in_one_step() {
    let (store, owner) = store_with_owner().await;
    let admin = store
        .create(&email("mia@example.com"), "Mia", Role::Admin, at())
        .await
        .expect("made");
    assert!(
        !store
            .transfer(admin.id(), owner.id())
            .await
            .expect("written"),
        "only the owner hands over"
    );
    assert!(
        !store
            .transfer(owner.id(), owner.id())
            .await
            .expect("written")
    );
    assert!(
        store
            .transfer(owner.id(), admin.id())
            .await
            .expect("written")
    );
    let old = store.find(owner.id()).await.expect("read").expect("kept");
    let new = store.find(admin.id()).await.expect("read").expect("kept");
    assert_eq!((old.role(), new.role()), (Role::Admin, Role::Owner));
}

#[tokio::test]
async fn a_password_is_stored_and_a_removed_account_is_gone() {
    let (store, owner) = store_with_owner().await;
    let hash = PasswordHash::from_phc("$argon2id$v=19$m=8,t=1,p=1$c2FsdHNhbHQ$aGFzaA".to_owned())
        .expect("an Argon2id hash");
    assert!(
        store
            .set_password(owner.id(), &hash)
            .await
            .expect("written")
    );
    let found = store
        .find_by_email(owner.email())
        .await
        .expect("read")
        .expect("there");
    assert_eq!(found.password, Some(hash));
    assert!(store.remove(owner.id()).await.expect("written"));
    assert!(
        !store.remove(owner.id()).await.expect("written"),
        "already gone"
    );
    assert_eq!(store.find(owner.id()).await.expect("read"), None);
}
