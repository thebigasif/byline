//! What every passkey store must do, shown on the one in memory. The
//! SQLite store in `khalid-db` runs the same cases.

use chrono::{DateTime, TimeZone, Utc};
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
