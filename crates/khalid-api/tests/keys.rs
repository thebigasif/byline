//! API keys: the token format, and checking a content key against the
//! key store without revealing anything through errors or logs.

use khalid_api::keys::{
    KeyError, KeyId, KeyKind, KeyToken, MemoryKeyStore, StoredKey, authenticate,
};
use proptest::prelude::{prop_assert, prop_assert_eq, proptest};

/// A key id and secret made up for these tests. They open nothing.
const ID: &str = "0123456789abcdef";
const SECRET: &str = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";

#[cfg(test)]
fn token(prefix: &str, id: &str, secret: &str) -> String {
    format!("{prefix}_{id}_{secret}")
}

#[cfg(test)]
fn store() -> MemoryKeyStore {
    let token: KeyToken = token("kc", ID, SECRET).parse().unwrap();
    MemoryKeyStore::new(vec![StoredKey::for_token(&token)])
}

#[test]
fn a_content_key_reads_its_kind_and_id() {
    let parsed: KeyToken = token("kc", ID, SECRET).parse().unwrap();
    assert_eq!(parsed.kind(), KeyKind::Content);
    assert_eq!(parsed.id().as_str(), ID);
}

#[test]
fn malformed_keys_are_refused() {
    let short = SECRET.get(..63).unwrap();
    for text in [
        String::new(),
        token("kx", ID, SECRET),
        token("kc", "0123456789ABCDEF", SECRET),
        token("kc", "0123", SECRET),
        token("kc", ID, short),
        format!("kc_{ID}{SECRET}"),
        format!("{} ", token("kc", ID, SECRET)),
    ] {
        assert!(text.parse::<KeyToken>().is_err(), "{text:?}");
    }
}

#[test]
fn the_secret_never_appears_in_debug_output() {
    let parsed: KeyToken = token("kc", ID, SECRET).parse().unwrap();
    let debug = format!("{parsed:?} {:?}", StoredKey::for_token(&parsed));
    assert!(!debug.contains(SECRET), "{debug}");
    assert!(!debug.contains(SECRET.get(..16).unwrap()), "{debug}");
}

#[tokio::test]
async fn the_right_content_key_is_accepted() {
    let id = authenticate(&store(), &token("kc", ID, SECRET), KeyKind::Content).await;
    assert_eq!(id.unwrap(), KeyId::try_new(ID).unwrap());
}

#[tokio::test]
async fn a_wrong_or_unknown_key_gets_the_same_answer() {
    let other_secret = SECRET.replace('f', "e");
    let cases = [
        token("kc", ID, &other_secret),
        token("kc", "fedcba9876543210", SECRET),
        "not a key at all".to_owned(),
    ];
    for text in cases {
        let error = authenticate(&store(), &text, KeyKind::Content)
            .await
            .unwrap_err();
        assert!(matches!(error, KeyError::Rejected), "{text}");
        assert_eq!(error.to_string(), "the key was not accepted");
    }
}

proptest! {
    #[test]
    fn a_stored_key_matches_only_its_own_secret(secret in "[0-9a-f]{64}", other in "[0-9a-f]{64}") {
        let mine: KeyToken = token("kc", ID, &secret).parse().unwrap();
        let theirs: KeyToken = token("kc", ID, &other).parse().unwrap();
        let stored = StoredKey::for_token(&mine);
        prop_assert!(stored.matches(&mine));
        prop_assert_eq!(stored.matches(&theirs), secret == other);
    }
}
