//! Making and revoking keys: 256 random bits, shown once, stored hashed
//! (RULES.md BL-SEC-3, ADR 0011). `khalid keys create` and `revoke` call
//! these.

use khalid_api::keys::{
    KeyError, KeyHashing, KeyKind, KeyStore, KeyToken, MemoryKeyStore, Scope, StoredKey,
    authenticate,
};

/// Random bytes made up for these tests: 8 for the identity, then 32.
const BYTES: [u8; 40] = [
    0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
    0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x0f, 0x1e, 0x2d, 0x3c, 0x4b, 0x5a, 0x69, 0x78,
    0x87, 0x96, 0xa5, 0xb4, 0xc3, 0xd2, 0xe1, 0xf0,
];

#[test]
fn random_bytes_become_a_key_that_reads_back() {
    let token = KeyToken::from_random(KeyKind::Integration, BYTES);
    assert_eq!(token.id().as_str(), "0123456789abcdef");
    let text = token.reveal();
    assert_eq!(
        text.expose(),
        "ki_0123456789abcdef_00112233445566778899aabbccddeeff0f1e2d3c4b5a69788796a5b4c3d2e1f0"
    );
    let again: KeyToken = text.expose().parse().unwrap();
    assert_eq!(again.kind(), KeyKind::Integration);
}

#[test]
fn each_kind_gets_its_prefix() {
    for (kind, prefix) in [
        (KeyKind::Content, "kc_"),
        (KeyKind::Integration, "ki_"),
        (KeyKind::Staff, "ks_"),
    ] {
        let text = KeyToken::from_random(kind, BYTES).reveal();
        assert!(text.expose().starts_with(prefix), "{kind:?}");
    }
}

#[test]
fn generated_keys_differ_and_never_show_in_debug() {
    let first = KeyToken::generate(KeyKind::Staff).unwrap();
    let second = KeyToken::generate(KeyKind::Staff).unwrap();
    assert_ne!(first.id(), second.id());
    let debug = format!("{first:?}");
    let text = first.reveal();
    let (_, secret) = text.expose().rsplit_once('_').unwrap();
    assert!(!debug.contains(secret), "the secret is in {debug}");
    assert_eq!(format!("{text:?}"), "[redacted]");
    assert_ne!(text.expose(), second.reveal().expose());
}

#[tokio::test]
async fn a_created_key_opens_until_it_is_revoked() {
    let store = MemoryKeyStore::default();
    let token = KeyToken::from_random(KeyKind::Integration, BYTES);
    let stored = StoredKey::for_integration(
        &token,
        vec![Scope::PostsWrite],
        &KeyHashing::fast_for_tests(),
    )
    .unwrap();
    store.create(stored, "Zapier").await.unwrap();
    let id = token.id().clone();
    let text = token.reveal();
    let principal = authenticate(&store, text.expose(), KeyKind::Integration)
        .await
        .unwrap();
    assert!(principal.access().allows(Scope::PostsWrite));

    assert!(store.revoke(&id).await.unwrap(), "the key was there");
    let refused = authenticate(&store, text.expose(), KeyKind::Integration).await;
    assert!(matches!(refused, Err(KeyError::Rejected)), "{refused:?}");
    assert!(!store.revoke(&id).await.unwrap(), "revoked twice");
}

#[tokio::test]
async fn an_identity_already_stored_is_refused() {
    let store = MemoryKeyStore::default();
    let stored = StoredKey::for_token(&KeyToken::from_random(KeyKind::Content, BYTES));
    store.create(stored.clone(), "Zapier").await.unwrap();
    assert!(store.create(stored, "Zapier").await.is_err());
}
