//! What every passkey store must do, shown on the one in memory. The
//! SQLite store in `khalid-db` runs the same cases.

use chrono::{DateTime, TimeDelta, TimeZone, Utc};
use khalid_auth::{AccountId, MemoryPasskeys, PasskeyStore, SavedPasskey};

#[cfg(test)]
fn at() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap()
}

/// A passkey with credential `[id; 16]` at counter `count`. A store never
/// reads the JSON, so any JSON does.
#[cfg(test)]
fn passkey(id: u8, count: u32) -> SavedPasskey {
    let json = serde_json::json!({ "cred": { "counter": count } }).to_string();
    SavedPasskey::from_store(vec![id; 16], json)
}

#[tokio::test]
async fn a_passkey_is_found_by_its_credential_and_listed_with_its_account() {
    let store = MemoryPasskeys::new();
    let (ana, ben) = (AccountId::from_row(1), AccountId::from_row(2));
    for (account, id, name) in [(ana, 1, "Laptop"), (ana, 2, "Phone"), (ben, 3, "Key")] {
        let added = store.add(account, &passkey(id, 3), name, at()).await;
        assert!(added.unwrap(), "{name}");
    }
    let found = store.find(&[1; 16]).await.unwrap().unwrap();
    assert_eq!(
        (found.account, found.name.as_str(), found.passkey),
        (ana, "Laptop", passkey(1, 3))
    );
    assert_eq!(store.find(&[9; 16]).await.unwrap(), None);
    let names: Vec<String> = store
        .of(ana)
        .await
        .unwrap()
        .into_iter()
        .map(|p| p.name)
        .collect();
    assert_eq!(names, ["Laptop", "Phone"]);
}

#[tokio::test]
async fn a_credential_is_stored_once_and_its_new_counter_is_saved() {
    let store = MemoryPasskeys::new();
    let ana = AccountId::from_row(1);
    assert!(
        store
            .add(ana, &passkey(1, 3), "Laptop", at())
            .await
            .unwrap()
    );
    let again = store
        .add(AccountId::from_row(2), &passkey(1, 0), "Theirs", at())
        .await;
    assert!(!again.unwrap(), "the same credential twice");
    store.used(&passkey(1, 4), at()).await.unwrap();
    let found = store.find(&[1; 16]).await.unwrap().unwrap();
    assert_eq!(found.passkey, passkey(1, 4));
}

#[tokio::test]
async fn a_passkey_is_listed_renamed_and_removed_only_by_its_own_account() {
    let store = MemoryPasskeys::new();
    let (ana, ben) = (AccountId::from_row(1), AccountId::from_row(2));
    let later = at() + TimeDelta::days(2);
    store
        .add(ana, &passkey(1, 3), "Laptop", at())
        .await
        .unwrap();
    store.add(ana, &passkey(2, 3), "Phone", at()).await.unwrap();
    store.used(&passkey(1, 4), later).await.unwrap();
    let listed = store.listed(ana).await.unwrap();
    let shown: Vec<_> = listed
        .iter()
        .map(|p| (p.name.as_str(), p.added_at, p.last_used_at))
        .collect();
    assert_eq!(
        shown,
        [("Laptop", at(), Some(later)), ("Phone", at(), None)]
    );
    let (laptop, phone) = (listed[0].id, listed[1].id);
    let theirs = store.rename(ben, laptop, "Mine now").await.unwrap();
    assert!(!theirs, "only Ana renames her own");
    assert!(store.rename(ana, laptop, "Work laptop").await.unwrap());
    assert!(
        !store.remove(ben, phone).await.unwrap(),
        "only Ana removes her own"
    );
    assert!(store.remove(ana, phone).await.unwrap());
    assert!(!store.remove(ana, phone).await.unwrap(), "already gone");
    let listed = store.listed(ana).await.unwrap();
    let names: Vec<&str> = listed.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["Work laptop"]);
    assert_eq!(
        store.find(&[2; 16]).await.unwrap(),
        None,
        "it signs nobody in"
    );
}
