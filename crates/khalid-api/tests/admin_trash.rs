//! The trash through the Admin API (KH-EDT-6, brief G52): a deleted post
//! is listed, restored where it was, or deleted for good, and a live post
//! needs `posts:publish` to come back.

use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use chrono::{TimeZone, Utc};
use khalid_api::api;
use khalid_api::content_store::MemoryContentStore;
use khalid_api::keys::{KeyHashing, KeyToken, MemoryKeyStore, Scope, StoredKey};
use khalid_api::post_store::{MemoryPostStore, PostStore};
use khalid_api::publish_door::{PublishDoor, Silent};
use khalid_api::resources::AdminPost;
use khalid_core::{FixedClock, Slug};
use serde_json::{Value, json};
use tower::ServiceExt;

/// Keys made up for these tests. They open nothing real.
const WRITER: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const PUBLISHER: &str =
    "ki_2222333344445555_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";

/// When both posts last changed, and when every delete happens.
const LAST_CHANGE: &str = "2026-10-20T08:30:00Z";

#[cfg(test)]
fn router() -> Router {
    let hashing = KeyHashing::fast_for_tests();
    let token = |text: &str| text.parse::<KeyToken>().unwrap();
    let key = |text, scopes| StoredKey::for_integration(&token(text), scopes, &hashing).unwrap();
    let keys = MemoryKeyStore::new(vec![
        key(WRITER, vec![Scope::PostsWrite]),
        key(PUBLISHER, vec![Scope::PostsWrite, Scope::PostsPublish]),
    ]);
    let at = Utc.with_ymd_and_hms(2026, 10, 20, 8, 30, 0).unwrap();
    let post = |id, slug| AdminPost::new(id, Slug::try_new(slug).unwrap(), "A post", "Text.", at);
    let live = post("en.live", "live").published(at);
    let posts = MemoryPostStore::new(vec![post("en.draft", "draft"), live]);
    let posts = Arc::new(posts.with_clock(FixedClock::new(at)));
    let store: Arc<dyn PostStore> = Arc::<MemoryPostStore>::clone(&posts);
    let door = PublishDoor::new(store, Arc::new(Silent));
    let content = Arc::new(MemoryContentStore::new(Vec::new()));
    let (router, _) = api::router_with_trash(content, Arc::new(door), Arc::new(keys), posts);
    router
}

#[cfg(test)]
async fn send(router: &Router, key: &str, (method, path): (&str, &str)) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(method)
        .uri(format!("/api/admin/v1{path}"))
        .header("authorization", format!("Khalid {key}"))
        .header("content-type", "application/json")
        .body(Body::empty())
        .unwrap();
    let response = router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// Deletes post `id` with `key`.
#[cfg(test)]
async fn delete(router: &Router, key: &str, id: &str) {
    let path = format!("/posts/{id}/?updated_at={LAST_CHANGE}");
    let (status, json) = send(router, key, ("DELETE", &path)).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "{json}");
}

#[tokio::test]
async fn a_deleted_post_is_listed_newest_first_with_the_status_it_had() {
    let router = router();
    let (status, json) = send(&router, WRITER, ("GET", "/trash/")).await;
    assert_eq!((status, json), (StatusCode::OK, json!({"trash": []})));
    delete(&router, WRITER, "en.draft").await;
    delete(&router, PUBLISHER, "en.live").await;
    let (status, json) = send(&router, WRITER, ("GET", "/trash/")).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    let listed = json!({"trash": [
        {"key": "20261020T083000Z-en.live", "id": "en.live",
            "title": "A post", "status": "published", "trashed_at": LAST_CHANGE},
        {"key": "20261020T083000Z-en.draft", "id": "en.draft",
            "title": "A post", "status": "draft", "trashed_at": LAST_CHANGE},
    ]});
    assert_eq!(json, listed);
}
