//! The trash a post store keeps (KH-EDT-6, brief G52): a deleted post
//! waits there, comes back where it was unless its address is taken, and
//! can be deleted for good. `MemoryPostStore` is checked here; `khalid
//! serve`'s files are checked in the binary's tests.

use chrono::{TimeZone, Utc};
use khalid_api::post_store::{MemoryPostStore, PostStore};
use khalid_api::post_trash::{self, PostTrash, TrashError};
use khalid_api::resources::{AdminPost, PostDraft, PostStatus};
use khalid_core::{FixedClock, Slug};

#[tokio::test]
async fn a_deleted_post_waits_in_the_trash_until_restored_or_purged() {
    let at = Utc.with_ymd_and_hms(2026, 10, 20, 8, 30, 0).unwrap();
    let post = |id, slug| AdminPost::new(id, Slug::try_new(slug).unwrap(), "A post", "Text.", at);
    let posts = MemoryPostStore::new(vec![post("hello", "hello"), post("links", "links")])
        .with_clock(FixedClock::new(at));
    posts.delete("hello", at).await.unwrap();
    posts.delete("links", at).await.unwrap();
    let keys: Vec<String> = posts
        .trashed()
        .await
        .unwrap()
        .into_iter()
        .map(|t| t.key)
        .collect();
    assert_eq!(
        keys,
        [post_trash::key(at, "links"), post_trash::key(at, "hello")]
    );
    assert_eq!(keys[1], "20261020T083000Z-hello");

    let draft = PostDraft::new("Hello again", Some(Slug::try_new("hello").unwrap()), "New.");
    posts.create(draft, "test", None).await.unwrap();
    let taken = posts.restore(&keys[1]).await;
    assert!(matches!(taken, Err(TrashError::Taken { id }) if id == "hello"));
    let back = posts.restore(&keys[0]).await.unwrap();
    assert_eq!((back.id(), back.status()), ("links", PostStatus::Draft));
    assert!(posts.get("links").await.unwrap().is_some());

    posts.purge(&keys[1]).await.unwrap();
    assert!(matches!(
        posts.purge(&keys[1]).await,
        Err(TrashError::NotFound)
    ));
    assert_eq!(posts.trashed().await.unwrap(), []);
}
