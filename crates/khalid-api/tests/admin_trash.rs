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

#[tokio::test]
async fn a_deleted_draft_is_restored_where_it_was() {
    let router = router();
    delete(&router, WRITER, "en.draft").await;
    let restore = ("POST", "/trash/20261020T083000Z-en.draft/restore/");
    let (status, json) = send(&router, WRITER, restore).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["posts"][0]["id"], "en.draft", "{json}");
    let (status, _) = send(&router, WRITER, ("GET", "/posts/en.draft/")).await;
    assert_eq!(status, StatusCode::OK);
    let (_, json) = send(&router, WRITER, ("GET", "/trash/")).await;
    assert_eq!(json, json!({"trash": []}));
    let (status, json) = send(&router, WRITER, restore).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{json}");
}

#[tokio::test]
async fn a_live_post_comes_back_only_with_posts_publish() {
    let router = router();
    delete(&router, PUBLISHER, "en.live").await;
    let restore = ("POST", "/trash/20261020T083000Z-en.live/restore/");
    let (status, json) = send(&router, WRITER, restore).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{json}");
    let (status, json) = send(&router, PUBLISHER, restore).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["posts"][0]["status"], "published", "{json}");
}

#[tokio::test]
async fn a_post_whose_address_is_taken_stays_in_the_trash() {
    let router = router();
    delete(&router, WRITER, "en.draft").await;
    let request = Request::builder()
        .method("POST")
        .uri("/api/admin/v1/posts/")
        .header("authorization", format!("Khalid {WRITER}"))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"posts": [{"title": "Again", "slug": "draft", "markdown": "New."}]}).to_string(),
        ))
        .unwrap();
    let made = router.clone().oneshot(request).await.unwrap();
    assert_eq!(made.status(), StatusCode::CREATED);
    let restore = ("POST", "/trash/20261020T083000Z-en.draft/restore/");
    let (status, json) = send(&router, WRITER, restore).await;
    assert_eq!(status, StatusCode::CONFLICT, "{json}");
    assert!(json["fix"].as_str().unwrap().contains("slug"), "{json}");
    let (_, json) = send(&router, WRITER, ("GET", "/trash/")).await;
    assert_eq!(json["trash"].as_array().map(Vec::len), Some(1), "{json}");
}

#[tokio::test]
async fn deleting_for_good_needs_posts_publish_and_empties_the_trash() {
    let router = router();
    delete(&router, WRITER, "en.draft").await;
    let purge = ("DELETE", "/trash/20261020T083000Z-en.draft/");
    let (status, json) = send(&router, WRITER, purge).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{json}");
    let (status, json) = send(&router, PUBLISHER, purge).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "{json}");
    let (status, json) = send(&router, PUBLISHER, purge).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{json}");
    let (_, json) = send(&router, WRITER, ("GET", "/trash/")).await;
    assert_eq!(json, json!({"trash": []}));
}
