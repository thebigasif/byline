//! Passkeys on SQLite keep the contract `khalid-auth` tests on its
//! in-memory store, and end with their account.

use std::path::PathBuf;

use chrono::{DateTime, TimeZone, Utc};
use khalid_api::keys::Role;
use khalid_auth::{Account, AccountStore, PasskeyStore, PublicKey, Registration};
use khalid_core::Slug;
use khalid_db::{AccountTable, Database, PasskeyTable};

#[cfg(test)]
fn at() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap()
}

#[cfg(test)]
fn registration(id: u8, public_key: PublicKey) -> Registration {
    Registration {
        credential_id: vec![id; 16],
        public_key,
        sign_count: 3,
    }
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
    let mut point = [7; 65];
    point[0] = 4;
    assert!(
        passkeys
            .add(
                ana.id(),
                &registration(1, PublicKey::Es256(point)),
                "Laptop",
                at()
            )
            .await
            .unwrap()
    );
    assert!(
        passkeys
            .add(
                ana.id(),
                &registration(2, PublicKey::Ed25519([5; 32])),
                "Phone",
                at()
            )
            .await
            .unwrap()
    );
    assert!(
        passkeys
            .add(
                ben.id(),
                &registration(3, PublicKey::Ed25519([6; 32])),
                "Key",
                at()
            )
            .await
            .unwrap()
    );
    let found = passkeys.find(&[1; 16]).await.unwrap().unwrap();
    assert_eq!(
        (found.account, found.public_key, found.sign_count),
        (ana.id(), PublicKey::Es256(point), 3)
    );
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
    let key = PublicKey::Ed25519([5; 32]);
    assert!(
        passkeys
            .add(ana.id(), &registration(1, key.clone()), "Laptop", at())
            .await
            .unwrap()
    );
    assert!(
        !passkeys
            .add(ben.id(), &registration(1, key), "Theirs", at())
            .await
            .unwrap()
    );
    passkeys.used(&[1; 16], 4, at()).await.unwrap();
    assert_eq!(
        passkeys.find(&[1; 16]).await.unwrap().unwrap().sign_count,
        4
    );
    accounts.remove(ana.id()).await.unwrap();
    assert_eq!(passkeys.find(&[1; 16]).await.unwrap(), None);
}
