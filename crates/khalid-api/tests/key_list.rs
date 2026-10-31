//! The keys a store lists: the ones that work, named, never with a hash
//! (KH-API-2, RULES.md BL-SEC-3).

use khalid_api::keys::{
    Access, KeyHashing, KeyKind, KeyStore, KeyToken, MemoryKeyStore, Scope, StoredKey,
};

#[tokio::test]
async fn memory_lists_the_working_keys_by_name() {
    let token = |first| {
        let mut bytes = [0x5a; 40];
        bytes[0] = first;
        KeyToken::from_random(KeyKind::Integration, bytes)
    };
    let hashing = KeyHashing::fast_for_tests();
    let key = |token: &KeyToken| {
        StoredKey::for_integration(token, vec![Scope::PostsWrite], &hashing).unwrap()
    };
    let (kept, serpdad, gone) = (token(1), token(2), token(3));
    let store = MemoryKeyStore::new(vec![key(&kept)]);
    store.create(key(&serpdad), "SerpDad").await.unwrap();
    store.create(key(&gone), "Old site").await.unwrap();
    store.revoke(gone.id()).await.unwrap();

    let listed = store.list().await.unwrap();
    let names: Vec<_> = listed.iter().map(|key| key.name.as_str()).collect();
    assert_eq!(names, [kept.id().as_str(), "SerpDad"]);
    assert_eq!(
        listed[1].access,
        Access::Integration(vec![Scope::PostsWrite])
    );
}
