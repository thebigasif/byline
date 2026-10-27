//! What every passkey store must do, shown on the one in memory. The
//! SQLite store in `khalid-db` runs the same cases.

use chrono::{DateTime, TimeZone, Utc};
use khalid_auth::{AccountId, MemoryPasskeys, PasskeyStore, PublicKey, Registration};

#[cfg(test)]
fn at() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap()
}

#[cfg(test)]
fn registration(id: u8) -> Registration {
    Registration {
        credential_id: vec![id; 16],
        public_key: PublicKey::Ed25519([id; 32]),
        sign_count: 3,
    }
}

#[tokio::test]
async fn a_passkey_is_found_by_its_credential_and_listed_with_its_account() {
    let store = MemoryPasskeys::new();
    let (ana, ben) = (AccountId::from_row(1), AccountId::from_row(2));
    assert!(
        store
            .add(ana, &registration(1), "Laptop", at())
            .await
            .unwrap()
    );
    assert!(
        store
            .add(ana, &registration(2), "Phone", at())
            .await
            .unwrap()
    );
    assert!(store.add(ben, &registration(3), "Key", at()).await.unwrap());
    let found = store.find(&[1; 16]).await.unwrap().unwrap();
    assert_eq!(
        (found.account, found.name.as_str(), found.sign_count),
        (ana, "Laptop", 3)
    );
    assert_eq!(found.public_key, PublicKey::Ed25519([1; 32]));
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
async fn a_credential_is_stored_once_and_its_counter_moves() {
    let store = MemoryPasskeys::new();
    let ana = AccountId::from_row(1);
    assert!(
        store
            .add(ana, &registration(1), "Laptop", at())
            .await
            .unwrap()
    );
    let again = store
        .add(AccountId::from_row(2), &registration(1), "Theirs", at())
        .await;
    assert!(!again.unwrap(), "the same credential twice");
    store.used(&[1; 16], 4, at()).await.unwrap();
    assert_eq!(store.find(&[1; 16]).await.unwrap().unwrap().sign_count, 4);
}
