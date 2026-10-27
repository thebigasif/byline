//! The webhook routes of the Admin API (task 1.19, KH-API-3): list and
//! switch off. Every route needs `webhooks:manage`.

mod common;

use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use chrono::{TimeZone, Utc};
use khalid_api::api;
use khalid_api::keys::{KeyHashing, KeyToken, MemoryKeyStore, Scope, StoredKey};
use khalid_api::post_store::MemoryPostStore;
use khalid_api::publish_door::{PublishDoor, Silent};
use khalid_api::webhook_store::{MemoryWebhookStore, WebhookStore};
use khalid_core::AbsoluteUrl;
use khalid_webhooks::Event;
use serde_json::{Value, json};
use tower::ServiceExt;

/// Keys made up for these tests. They open nothing real.
const MANAGER: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const POSTS_ONLY: &str =
    "ki_5555666677778888_0000111122223333444455556666777788889999aaaabbbbccccddddeeeeffff";

/// The API over `webhooks`, with a key that manages webhooks and one that
/// only writes posts.
#[cfg(test)]
fn router(webhooks: Arc<dyn WebhookStore>) -> axum::Router {
    let hashing = KeyHashing::fast_for_tests();
    let stored = |text: &str, scope| {
        let token: KeyToken = text.parse().unwrap();
        StoredKey::for_integration(&token, vec![scope], &hashing).unwrap()
    };
    let keys = MemoryKeyStore::new(vec![
        stored(MANAGER, Scope::WebhooksManage),
        stored(POSTS_ONLY, Scope::PostsWrite),
    ]);
    let door = PublishDoor::new(Arc::new(MemoryPostStore::default()), Arc::new(Silent));
    let content = Arc::new(common::store());
    let (router, _) = api::router_with_webhooks(content, Arc::new(door), Arc::new(keys), webhooks);
    router
}

/// A store holding one `comment.added` webhook, added at nine o'clock.
#[cfg(test)]
async fn store() -> Arc<dyn WebhookStore> {
    let at = Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap();
    let store = MemoryWebhookStore::new(at);
    let url = AbsoluteUrl::parse("https://hooks.example.com/khalid").unwrap();
    store.add(&url, Event::CommentAdded).await.unwrap();
    Arc::new(store)
}

/// Sends `method` to `path` with `key`; answers the status and the body
/// as JSON, or `null` when there is none.
#[cfg(test)]
async fn send(
    router: &axum::Router,
    key: &str,
    (method, path): (&str, &str),
) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(method)
        .uri(format!("/api/admin/v1{path}"))
        .header("authorization", format!("Khalid {key}"))
        .body(Body::empty())
        .unwrap();
    let response = router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn the_list_shows_each_webhook_without_its_secret() {
    let router = router(store().await);
    let (status, json) = send(&router, MANAGER, ("GET", "/webhooks/")).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(
        json,
        json!({"webhooks": [{"id": 1, "event": "comment.added",
            "url": "https://hooks.example.com/khalid",
            "created_at": "2026-10-27T09:00:00Z"}]})
    );
}

#[tokio::test]
async fn a_webhook_switched_off_is_gone_and_a_second_try_is_a_404() {
    let router = router(store().await);
    let (status, _) = send(&router, MANAGER, ("DELETE", "/webhooks/1/")).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, json) = send(&router, MANAGER, ("GET", "/webhooks/")).await;
    assert_eq!(json, json!({"webhooks": []}));
    for path in ["/webhooks/1/", "/webhooks/first/"] {
        let (status, json) = send(&router, MANAGER, ("DELETE", path)).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{path}: {json}");
        let detail = json["detail"].as_str().unwrap_or_default();
        assert!(
            detail.starts_with("No webhook that is on has this identity."),
            "{json}"
        );
    }
}

#[tokio::test]
async fn a_key_without_webhooks_manage_is_refused() {
    let router = router(store().await);
    for (method, path) in [("GET", "/webhooks/"), ("DELETE", "/webhooks/1/")] {
        let (status, json) = send(&router, POSTS_ONLY, (method, path)).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{method} {path}: {json}");
    }
    let (_, json) = send(&router, MANAGER, ("GET", "/webhooks/")).await;
    assert_eq!(json["webhooks"].as_array().map(Vec::len), Some(1));
}

#[tokio::test]
async fn the_document_lists_the_webhook_routes() {
    let request = Request::get("/api/openapi.json").body(Body::empty());
    let response = router(store().await).oneshot(request.unwrap()).await;
    let bytes = to_bytes(response.unwrap().into_body(), 4 * 1024 * 1024).await;
    let served: Value = serde_json::from_slice(&bytes.unwrap()).unwrap();
    let paths = &served["paths"];
    assert!(paths["/api/admin/v1/webhooks/"]["get"].is_object());
    assert!(paths["/api/admin/v1/webhooks/{id}/"]["delete"].is_object());
}
