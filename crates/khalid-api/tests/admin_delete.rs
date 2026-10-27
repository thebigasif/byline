//! Deleting a post through the Admin API, and the 409 a stale delete gets
//! (RULES.md BL-API-7, KH-API-2).

use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use chrono::{TimeZone, Utc};
use khalid_api::api;
use khalid_api::content_store::MemoryContentStore;
use khalid_api::keys::{KeyHashing, KeyToken, MemoryKeyStore, Scope, StoredKey};
use khalid_api::post_store::MemoryPostStore;
use khalid_api::resources::AdminPost;
use khalid_core::Slug;
use serde_json::Value;
use tower::ServiceExt;

/// Keys made up for these tests. They open nothing real.
const WRITER: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const PUBLISHER: &str =
    "ki_2222333344445555_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const READER: &str =
    "ki_3333444455556666_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";

/// When both posts last changed.
const LAST_CHANGE: &str = "2026-10-20T08:30:00Z";

#[cfg(test)]
fn router() -> Router {
    let hashing = KeyHashing::fast_for_tests();
    let token = |text: &str| text.parse::<KeyToken>().unwrap();
    let key = |text, scopes| StoredKey::for_integration(&token(text), scopes, &hashing).unwrap();
    let keys = MemoryKeyStore::new(vec![
        key(WRITER, vec![Scope::PostsWrite]),
        key(PUBLISHER, vec![Scope::PostsWrite, Scope::PostsPublish]),
        key(READER, vec![Scope::ContentRead]),
    ]);
    let at = Utc.with_ymd_and_hms(2026, 10, 20, 8, 30, 0).unwrap();
    let post = |id, slug| AdminPost::new(id, Slug::try_new(slug).unwrap(), "A post", "Text.", at);
    let live = post("en.live", "live").published(at);
    let posts = MemoryPostStore::new(vec![post("en.draft", "draft"), live]);
    let content = MemoryContentStore::new(Vec::new());
    let (router, _) = api::router(Arc::new(content), Arc::new(posts), Arc::new(keys));
    router
}

#[cfg(test)]
async fn send(router: &Router, method: &str, uri: &str, key: Option<&str>) -> (StatusCode, Value) {
    let mut request = Request::builder().method(method).uri(uri);
    if let Some(key) = key {
        request = request.header("authorization", format!("Khalid {key}"));
    }
    let request = request.body(Body::empty()).unwrap();
    let response = router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[cfg(test)]
fn delete_uri(id: &str, updated_at: &str) -> String {
    format!("/api/admin/v1/posts/{id}/?updated_at={updated_at}")
}

#[tokio::test]
async fn a_draft_deleted_on_its_latest_copy_is_gone() {
    let router = router();
    let uri = delete_uri("en.draft", LAST_CHANGE);
    let (status, json) = send(&router, "DELETE", &uri, Some(WRITER)).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "{json}");
    let read = send(
        &router,
        "GET",
        "/api/admin/v1/posts/en.draft/",
        Some(WRITER),
    )
    .await;
    assert_eq!(read.0, StatusCode::NOT_FOUND, "{}", read.1);
}

#[tokio::test]
async fn a_stale_delete_is_a_409_and_keeps_the_post() {
    let router = router();
    let uri = delete_uri("en.draft", "2026-10-19T12:00:00Z");
    let (status, json) = send(&router, "DELETE", &uri, Some(WRITER)).await;
    assert_eq!(status, StatusCode::CONFLICT, "{json}");
    assert_eq!(json["rule"], "updated_at", "{json}");
    let read = send(
        &router,
        "GET",
        "/api/admin/v1/posts/en.draft/",
        Some(WRITER),
    )
    .await;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
}

#[tokio::test]
async fn a_delete_without_updated_at_is_a_400() {
    let router = router();
    let uri = "/api/admin/v1/posts/en.draft/";
    let (status, json) = send(&router, "DELETE", uri, Some(WRITER)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{json}");
    assert_eq!(json["rule"], "updated_at", "{json}");
    assert!(
        json["fix"].as_str().unwrap().contains("updated_at"),
        "{json}"
    );
}

#[tokio::test]
async fn deleting_needs_a_key_with_posts_write() {
    let router = router();
    let uri = delete_uri("en.draft", LAST_CHANGE);
    let (status, _) = send(&router, "DELETE", &uri, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, json) = send(&router, "DELETE", &uri, Some(READER)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{json}");
    assert_eq!(json["rule"], "posts:write", "{json}");
}

#[tokio::test]
async fn deleting_a_live_post_needs_posts_publish() {
    let router = router();
    let uri = delete_uri("en.live", LAST_CHANGE);
    let (status, json) = send(&router, "DELETE", &uri, Some(WRITER)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{json}");
    assert_eq!(json["rule"], "posts:publish", "{json}");
    let (status, json) = send(&router, "DELETE", &uri, Some(PUBLISHER)).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "{json}");
}

#[tokio::test]
async fn deleting_an_unknown_post_is_a_404() {
    let router = router();
    let uri = delete_uri("en.nothing", LAST_CHANGE);
    let (status, json) = send(&router, "DELETE", &uri, Some(PUBLISHER)).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{json}");
}
