//! API keys on SQLite: a stored key opens until it is revoked, each kind
//! reads back with what it may do, the secret never reaches the file, and
//! each site sees only its own keys (RULES.md BL-SEC-3, ADR 0011).

use std::path::PathBuf;

use chrono::{TimeZone, Utc};
use khalid_api::keys::{
    Access, CreateKeyError, KeyError, KeyHashing, KeyKind, KeyStore, KeyToken, Role, Scope,
    StoredKey, authenticate,
};
use khalid_core::{FixedClock, Slug};
use khalid_db::{Database, KeyTable};

/// A migrated database with the sites `demo` and `other`, and the key
/// table of each.
#[cfg(test)]
async fn opened() -> (tempfile::TempDir, PathBuf, KeyTable, KeyTable) {
    let dir = tempfile::tempdir().unwrap();
    let mut path = PathBuf::from(dir.path());
    path.push("site.db");
    let db = Database::open(&path).await.unwrap();
    db.migrate().await.unwrap();
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 10, 26, 9, 0, 0).unwrap());
    let demo = db.site(&Slug::try_new("demo").unwrap()).await.unwrap();
    let other = db.site(&Slug::try_new("other").unwrap()).await.unwrap();
    let demo = KeyTable::new(db.clone(), demo, clock);
    let other = KeyTable::new(db, other, clock);
    (dir, path, demo, other)
}

/// A key of `kind` made from bytes that start with `first`.
#[cfg(test)]
fn token(kind: KeyKind, first: u8) -> KeyToken {
    let mut bytes = [0x5a; 40];
    bytes[0] = first;
    KeyToken::from_random(kind, bytes)
}

#[cfg(test)]
fn stored(token: &KeyToken) -> StoredKey {
    let hashing = KeyHashing::fast_for_tests();
    match token.kind() {
        KeyKind::Integration => {
            StoredKey::for_integration(token, vec![Scope::ContentRead, Scope::PostsWrite], &hashing)
                .unwrap()
        }
        KeyKind::Staff => StoredKey::for_staff(token, Role::Editor, &hashing).unwrap(),
        _ => StoredKey::for_token(token),
    }
}

#[tokio::test]
async fn each_kind_reads_back_with_what_it_may_do() {
    let (_dir, _path, keys, _) = opened().await;
    for (kind, first) in [
        (KeyKind::Content, 1),
        (KeyKind::Integration, 2),
        (KeyKind::Staff, 3),
    ] {
        let token = token(kind, first);
        let key = stored(&token);
        keys.create(key.clone(), "SerpDad").await.unwrap();
        let found = keys.find(token.id()).await.unwrap();
        assert_eq!(found.as_ref(), Some(&key), "{kind:?}");
        let principal = authenticate(&keys, token.reveal().expose(), kind)
            .await
            .unwrap();
        assert_eq!(principal.access(), key.access());
    }
}

#[tokio::test]
async fn scopes_read_back_once_each_in_the_standard_order() {
    let (_dir, _path, keys, _) = opened().await;
    let token = token(KeyKind::Integration, 8);
    let scopes = vec![Scope::PostsWrite, Scope::ContentRead, Scope::PostsWrite];
    let key = StoredKey::for_integration(&token, scopes, &KeyHashing::fast_for_tests()).unwrap();
    keys.create(key, "SerpDad").await.unwrap();
    let found = keys.find(token.id()).await.unwrap().unwrap();
    assert_eq!(
        found.access(),
        &Access::Integration(vec![Scope::ContentRead, Scope::PostsWrite])
    );
}

#[tokio::test]
async fn a_key_opens_until_it_is_revoked_and_its_identity_stays_taken() {
    let (_dir, _path, keys, _) = opened().await;
    let token = token(KeyKind::Integration, 4);
    let key = stored(&token);
    keys.create(key.clone(), "Zapier").await.unwrap();
    assert!(keys.revoke(token.id()).await.unwrap(), "the key was there");
    assert!(!keys.revoke(token.id()).await.unwrap(), "revoked twice");
    let refused = authenticate(&keys, token.reveal().expose(), KeyKind::Integration).await;
    assert!(matches!(refused, Err(KeyError::Rejected)), "{refused:?}");

    let again = keys.create(key, "Zapier").await;
    assert!(
        matches!(again, Err(CreateKeyError::IdTaken(_))),
        "{again:?}"
    );
}

#[tokio::test]
async fn the_file_holds_hashes_and_never_the_secret() {
    let (_dir, path, keys, _) = opened().await;
    let (content, integration) = (token(KeyKind::Content, 5), token(KeyKind::Integration, 6));
    keys.create(stored(&content), "Web page").await.unwrap();
    keys.create(stored(&integration), "SerpDad").await.unwrap();
    let file = std::fs::read(&path).unwrap();
    let mut wal = path.into_os_string();
    wal.push("-wal");
    let file = [file, std::fs::read(wal).unwrap_or_default()].concat();
    for token in [content, integration] {
        let text = token.reveal();
        let (_, secret) = text.expose().rsplit_once('_').unwrap();
        let found = file.windows(secret.len()).any(|w| w == secret.as_bytes());
        assert!(!found, "the secret of a key is in the database file");
    }
}

#[tokio::test]
async fn a_site_sees_only_its_own_keys() {
    let (_dir, _path, demo, other) = opened().await;
    let token = token(KeyKind::Staff, 7);
    demo.create(stored(&token), "Lena").await.unwrap();
    assert!(other.find(token.id()).await.unwrap().is_none());
    assert!(!other.revoke(token.id()).await.unwrap());
    assert!(demo.find(token.id()).await.unwrap().is_some());
}

#[tokio::test]
async fn the_working_keys_are_listed_with_names_and_never_a_hash() {
    let (_dir, _path, demo, other) = opened().await;
    let serpdad = token(KeyKind::Integration, 1);
    let editor = token(KeyKind::Staff, 2);
    let gone = token(KeyKind::Content, 3);
    demo.create(stored(&serpdad), "SerpDad").await.unwrap();
    demo.create(stored(&editor), "Lena").await.unwrap();
    demo.create(stored(&gone), "Old site").await.unwrap();
    demo.revoke(gone.id()).await.unwrap();
    other
        .create(stored(&token(KeyKind::Content, 4)), "Elsewhere")
        .await
        .unwrap();

    let listed = demo.list().await.unwrap();
    let names: Vec<_> = listed.iter().map(|key| key.name.as_str()).collect();
    assert_eq!(
        names,
        ["SerpDad", "Lena"],
        "revoked and other sites' keys are left out"
    );
    assert_eq!(listed[0].id, *serpdad.id());
    assert_eq!(
        listed[0].access,
        Access::Integration(vec![Scope::ContentRead, Scope::PostsWrite])
    );
    assert_eq!(listed[1].access, Access::Staff(Role::Editor));
    assert_eq!(
        listed[0].created_at,
        Utc.with_ymd_and_hms(2026, 10, 26, 9, 0, 0).unwrap()
    );
}

#[tokio::test]
async fn keys_made_in_the_same_second_are_listed_in_the_order_they_were_made() {
    let (_dir, _path, demo, _) = opened().await;
    let (owner, serpdad) = (
        token(KeyKind::Staff, 0xf0),
        token(KeyKind::Integration, 0x01),
    );
    assert!(
        serpdad.id().as_str() < owner.id().as_str(),
        "the later key sorts first by identity"
    );
    demo.create(stored(&owner), "Owner").await.unwrap();
    demo.create(stored(&serpdad), "SerpDad").await.unwrap();

    let listed = demo.list().await.unwrap();
    let names: Vec<_> = listed.iter().map(|key| key.name.as_str()).collect();
    assert_eq!(names, ["Owner", "SerpDad"]);
}
