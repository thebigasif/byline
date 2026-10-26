//! Integration keys and staff tokens (RULES.md BL-SEC-3, ADR 0011): stored
//! as Argon2id, compared in constant time, and carrying what they may do.

use khalid_api::keys::{
    Access, KeyHashing, KeyKind, KeyToken, MemoryKeyStore, Role, Scope, StoredKey, authenticate,
};

/// Keys made up for these tests. They open nothing.
const INTEGRATION: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const STAFF: &str =
    "ks_5555666677778888_0000111122223333444455556666777788889999aaaabbbbccccddddeeeeffff";

#[cfg(test)]
fn token(text: &str) -> KeyToken {
    text.parse().unwrap()
}

#[cfg(test)]
fn store() -> MemoryKeyStore {
    let hashing = KeyHashing::fast_for_tests();
    let scopes = vec![Scope::PostsWrite, Scope::MediaWrite];
    let integration = StoredKey::for_integration(&token(INTEGRATION), scopes, &hashing).unwrap();
    let staff = StoredKey::for_staff(&token(STAFF), Role::Editor, &hashing).unwrap();
    MemoryKeyStore::new(vec![integration, staff])
}

#[test]
fn an_argon2id_hash_is_stored_never_the_secret() {
    let stored = StoredKey::for_integration(
        &token(INTEGRATION),
        vec![Scope::PostsWrite],
        &KeyHashing::fast_for_tests(),
    )
    .unwrap();
    let debug = format!("{stored:?}");
    assert!(debug.contains("$argon2id$"), "{debug}");
    assert!(!debug.contains("aaaabbbbccccdddd"), "{debug}");
}

#[tokio::test]
async fn an_integration_key_carries_its_scopes() {
    let principal = authenticate(&store(), INTEGRATION, KeyKind::Integration)
        .await
        .unwrap();
    assert_eq!(principal.id().as_str(), "1111222233334444");
    let access = principal.access();
    assert!(access.allows(Scope::PostsWrite));
    assert!(access.allows(Scope::MediaWrite));
    assert!(!access.allows(Scope::SettingsWrite));
    assert!(!access.allows(Scope::ContentRead));
}

#[tokio::test]
async fn a_staff_token_carries_its_role() {
    let principal = authenticate(&store(), STAFF, KeyKind::Staff).await.unwrap();
    assert_eq!(principal.access(), &Access::Staff(Role::Editor));
    assert!(principal.access().allows(Scope::PostsWrite));
    assert!(!principal.access().allows(Scope::SettingsWrite));
}

#[tokio::test]
async fn a_wrong_secret_or_kind_is_refused() {
    let wrong = INTEGRATION.replace("ffff0000", "ffff0001");
    for (text, kind) in [
        (wrong.as_str(), KeyKind::Integration),
        (INTEGRATION, KeyKind::Staff),
        (STAFF, KeyKind::Integration),
    ] {
        assert!(
            authenticate(&store(), text, kind).await.is_err(),
            "{text} as {kind:?}"
        );
    }
}
