//! The `SerpDad` route of the Admin API (KH-API-6): off until a key is
//! linked. Every route needs `settings:write`.

mod common;

use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use chrono::{TimeZone, Utc};
use khalid_api::api;
use khalid_api::keys::{KeyHashing, KeyId, KeyToken, MemoryKeyStore, Scope, StoredKey};
use khalid_api::post_store::MemoryPostStore;
use khalid_api::publish_door::{PublishDoor, Silent};
use khalid_api::serpdad_store::{MemorySerpdadStore, SerpdadStore};
use serde_json::{Value, json};
use tower::ServiceExt;

/// Keys made up for these tests. They open nothing real.
const OWNER: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const WRITER: &str =
    "ki_5555666677778888_0000111122223333444455556666777788889999aaaabbbbccccddddeeeeffff";

/// The API with `serpdad`, a key that may change settings and one that
/// only writes posts.
#[cfg(test)]
fn router(serpdad: Option<Arc<dyn SerpdadStore>>) -> axum::Router {
    let hashing = KeyHashing::fast_for_tests();
    let stored = |text: &str, scope| {
        let token: KeyToken = text.parse().unwrap();
        StoredKey::for_integration(&token, vec![scope], &hashing).unwrap()
    };
    let keys = MemoryKeyStore::new(vec![
        stored(OWNER, Scope::SettingsWrite),
        stored(WRITER, Scope::PostsWrite),
    ]);
    let door = PublishDoor::new(Arc::new(MemoryPostStore::default()), Arc::new(Silent));
    let content = Arc::new(common::store());
    let (router, _) = api::router_with_serpdad(content, Arc::new(door), Arc::new(keys), serpdad);
    router
}

#[cfg(test)]
fn store() -> Arc<dyn SerpdadStore> {
    let at = Utc.with_ymd_and_hms(2026, 10, 31, 9, 40, 0).unwrap();
    Arc::new(MemorySerpdadStore::new(at))
}

/// Sends `method` to `path` with `key` and `body`; answers the status and
/// the body as JSON, or `null` when there is none.
#[cfg(test)]
async fn send(
    router: &axum::Router,
    key: &str,
    (method, path): (&str, &str),
    body: Option<&Value>,
) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(method)
        .uri(format!("/api/admin/v1{path}"))
        .header("authorization", format!("Khalid {key}"))
        .header("content-type", "application/json")
        .body(body.map_or_else(Body::empty, |body| Body::from(body.to_string())))
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
async fn the_connection_is_off_until_a_key_is_linked() {
    let router = router(Some(store()));
    let (status, json) = send(&router, OWNER, ("GET", "/serpdad/"), None).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json, json!({"connection": null}));
}

#[tokio::test]
async fn a_linked_key_shows_when_it_was_made() {
    let serpdad = store();
    let key = KeyId::try_new("9999aaaabbbbcccc").unwrap();
    serpdad.connect(&key).await.unwrap();
    let router = router(Some(serpdad));
    let (status, json) = send(&router, OWNER, ("GET", "/serpdad/"), None).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    let connected = json!({"connected_at": "2026-10-31T09:40:00Z"});
    assert_eq!(json, json!({"connection": connected}));
}

#[tokio::test]
async fn a_key_without_settings_write_is_refused() {
    let router = router(Some(store()));
    let (status, json) = send(&router, WRITER, ("GET", "/serpdad/"), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{json}");
    assert_eq!(json["rule"], "settings:write", "{json}");
}

#[tokio::test]
async fn a_site_with_no_database_is_off() {
    let router = router(None);
    let (_, read) = send(&router, OWNER, ("GET", "/serpdad/"), None).await;
    assert_eq!(read, json!({"connection": null}));
}

#[tokio::test]
async fn the_document_lists_the_serpdad_route() {
    let request = Request::get("/api/openapi.json").body(Body::empty());
    let response = router(Some(store())).oneshot(request.unwrap()).await;
    let bytes = to_bytes(response.unwrap().into_body(), 4 * 1024 * 1024).await;
    let served: Value = serde_json::from_slice(&bytes.unwrap()).unwrap();
    assert!(served["paths"]["/api/admin/v1/serpdad/"]["get"].is_object());
}
