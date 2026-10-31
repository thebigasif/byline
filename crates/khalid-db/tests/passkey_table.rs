//! Passkeys on SQLite keep the contract `khalid-auth` tests on its
//! in-memory store, and end with their account.

use std::path::PathBuf;

use chrono::{DateTime, TimeDelta, TimeZone, Utc};
use khalid_api::keys::Role;
use khalid_auth::{Account, AccountStore, PasskeyStore, SavedPasskey};
use khalid_core::Slug;
use khalid_db::{AccountTable, Database, PasskeyTable};

#[cfg(test)]
fn at() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap()
}

/// A passkey with credential `[id; 16]` at counter `count`. The table
/// keeps the JSON as it is given.
#[cfg(test)]
fn passkey(id: u8, count: u32) -> SavedPasskey {
    let json = format!(r#"{{"cred":{{"counter":{count}}}}}"#);
    SavedPasskey::from_store(vec![id; 16], json)
}

/// A migrated database with the sites `demo` and `other`, the passkeys of
/// each, the accounts of `demo`, and Ana and Ben on `demo`.
#[cfg(test)]
async fn opened() -> (
    tempfile::TempDir,
    PasskeyTable,
    PasskeyTable,
    AccountTable,
    [Account; 2],
) {
    let dir = tempfile::tempdir().unwrap();
    let mut path = PathBuf::from(dir.path());
    path.push("site.db");
    let db = Database::open(&path).await.unwrap();
    db.migrate().await.unwrap();
    let demo = db.site(&Slug::try_new("demo").unwrap()).await.unwrap();
    let other = db.site(&Slug::try_new("other").unwrap()).await.unwrap();
    let accounts = AccountTable::new(db.clone(), demo);
    let mut people = Vec::new();
    for name in ["ana", "ben"] {
        let email = format!("{name}@example.com").parse().unwrap();
        people.push(
            accounts
                .create(&email, name, Role::Writer, at())
                .await
                .unwrap(),
        );
    }
    let passkeys = PasskeyTable::new(db.clone(), demo);
    (
        dir,
        passkeys,
        PasskeyTable::new(db, other),
        accounts,
        people.try_into().unwrap(),
    )
}

#[tokio::test]
async fn a_passkey_reads_back_by_credential_and_by_account() {
    let (_dir, passkeys, other, _, [ana, ben]) = opened().await;
    for (account, id, name) in [(&ana, 1, "Laptop"), (&ana, 2, "Phone"), (&ben, 3, "Key")] {
        let added = passkeys
            .add(account.id(), &passkey(id, 3), name, at())
            .await;
        assert!(added.unwrap(), "{name}");
    }
    let found = passkeys.find(&[1; 16]).await.unwrap().unwrap();
    assert_eq!((found.account, found.passkey), (ana.id(), passkey(1, 3)));
    assert_eq!(
        other.find(&[1; 16]).await.unwrap(),
        None,
        "another site's passkey"
    );
    let names: Vec<String> = passkeys
        .of(ana.id())
        .await
        .unwrap()
        .into_iter()
        .map(|p| p.name)
        .collect();
    assert_eq!(names, ["Laptop", "Phone"]);
}

#[tokio::test]
async fn a_credential_is_stored_once_its_counter_moves_and_it_ends_with_its_account() {
    let (_dir, passkeys, _, accounts, [ana, ben]) = opened().await;
    let added = passkeys.add(ana.id(), &passkey(1, 3), "Laptop", at()).await;
    assert!(added.unwrap());
    let again = passkeys.add(ben.id(), &passkey(1, 0), "Theirs", at()).await;
    assert!(!again.unwrap(), "the same credential twice");
    passkeys.used(&passkey(1, 4), at()).await.unwrap();
    let found = passkeys.find(&[1; 16]).await.unwrap().unwrap();
    assert_eq!(found.passkey, passkey(1, 4));
    accounts.remove(ana.id()).await.unwrap();
    assert_eq!(passkeys.find(&[1; 16]).await.unwrap(), None);
}

#[tokio::test]
async fn the_table_refuses_a_passkey_that_is_not_json() {
    let (_dir, passkeys, _, _, [ana, _]) = opened().await;
    let broken = SavedPasskey::from_store(vec![1; 16], "not json".into());
    let refused = passkeys.add(ana.id(), &broken, "Laptop", at()).await;
    assert!(refused.is_err(), "{refused:?}");
}

#[tokio::test]
async fn a_passkey_is_listed_renamed_and_removed_only_by_its_own_account() {
    let (_dir, passkeys, other, _, [ana, ben]) = opened().await;
    let later = at() + TimeDelta::days(2);
    let (ana, ben) = (ana.id(), ben.id());
    passkeys
        .add(ana, &passkey(1, 3), "Laptop", at())
        .await
        .unwrap();
    passkeys
        .add(ana, &passkey(2, 3), "Phone", at())
        .await
        .unwrap();
    passkeys.used(&passkey(1, 4), later).await.unwrap();
    let listed = passkeys.listed(ana).await.unwrap();
    let shown: Vec<_> = listed
        .iter()
        .map(|p| (p.name.as_str(), p.added_at, p.last_used_at))
        .collect();
    assert_eq!(
        shown,
        [("Laptop", at(), Some(later)), ("Phone", at(), None)]
    );
    let (laptop, phone) = (listed[0].id, listed[1].id);
    assert!(!passkeys.rename(ben, laptop, "Mine now").await.unwrap());
    let elsewhere = other.rename(ana, laptop, "Mine now").await.unwrap();
    assert!(!elsewhere, "another site's table");
    assert!(passkeys.rename(ana, laptop, "Work laptop").await.unwrap());
    assert!(!passkeys.remove(ben, phone).await.unwrap());
    assert!(
        !other.remove(ana, phone).await.unwrap(),
        "another site's table"
    );
    assert!(passkeys.remove(ana, phone).await.unwrap());
    assert!(!passkeys.remove(ana, phone).await.unwrap(), "already gone");
    let listed = passkeys.listed(ana).await.unwrap();
    let names: Vec<&str> = listed.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["Work laptop"]);
    assert_eq!(passkeys.find(&[2; 16]).await.unwrap(), None);
}
