//! The `SerpDad` route of the Admin API (KH-API-6): off until a key is
//! made, the key shown once, a new key revoking the old, and turning off
//! refusing the key. Every route needs `settings:write`.

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

/// Stages a draft titled `title` with `key`, the way `SerpDad` does.
#[cfg(test)]
async fn stage(router: &axum::Router, key: &str, title: &str) -> StatusCode {
    let article = json!({"posts": [{"title": title, "html": "<p>Anchor text.</p>"}]});
    let route = ("POST", "/posts/?source=html");
    send(router, key, route, Some(&article)).await.0
}

/// Turns the connection on with the owner's key and answers the new key.
#[cfg(test)]
async fn connect(router: &axum::Router) -> String {
    let (status, made) = send(router, OWNER, ("POST", "/serpdad/"), None).await;
    assert_eq!(status, StatusCode::CREATED, "{made}");
    made["connection"]["key"].as_str().unwrap().to_owned()
}

#[tokio::test]
async fn turning_on_makes_a_key_that_writes_drafts_and_shows_it_once() {
    let router = router(Some(store()));
    let (status, made) = send(&router, OWNER, ("POST", "/serpdad/"), None).await;
    assert_eq!(status, StatusCode::CREATED, "{made}");
    let key = made["connection"]["key"].as_str().unwrap();
    assert!(key.starts_with("ki_"), "{made}");
    assert_eq!(made["connection"]["connected_at"], "2026-10-31T09:40:00Z");
    assert_eq!(
        stage(&router, key, "Choosing anchor text").await,
        StatusCode::CREATED
    );
    let (status, _) = send(&router, key, ("GET", "/serpdad/"), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "the key writes posts only");

    let (_, read) = send(&router, OWNER, ("GET", "/serpdad/"), None).await;
    assert_eq!(read["connection"]["connected_at"], "2026-10-31T09:40:00Z");
    assert_eq!(read["connection"].get("key"), None, "shown only once");
}

#[tokio::test]
async fn a_new_key_revokes_the_old_one() {
    let router = router(Some(store()));
    let old = connect(&router).await;
    let new = connect(&router).await;
    assert_ne!(old, new);
    assert_eq!(
        stage(&router, &old, "Too late").await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(stage(&router, &new, "On time").await, StatusCode::CREATED);
}

#[tokio::test]
async fn turning_off_refuses_the_key_and_a_second_time_is_a_404() {
    let router = router(Some(store()));
    let key = connect(&router).await;
    let (status, _) = send(&router, OWNER, ("DELETE", "/serpdad/"), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(
        stage(&router, &key, "Refused").await,
        StatusCode::UNAUTHORIZED
    );
    let (_, read) = send(&router, OWNER, ("GET", "/serpdad/"), None).await;
    assert_eq!(read["connection"], Value::Null);

    let (status, json) = send(&router, OWNER, ("DELETE", "/serpdad/"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{json}");
    let detail = json["detail"].as_str().unwrap_or_default();
    assert!(detail.starts_with("SerpDad is not connected."), "{json}");
}

#[tokio::test]
async fn a_key_without_settings_write_is_refused() {
    let router = router(Some(store()));
    for method in ["GET", "POST", "DELETE"] {
        let (status, json) = send(&router, WRITER, (method, "/serpdad/"), None).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{method}: {json}");
        assert_eq!(json["rule"], "settings:write", "{method}: {json}");
    }
}

#[tokio::test]
async fn a_site_with_no_database_is_off_and_says_why_it_cannot_connect() {
    let router = router(None);
    let (_, read) = send(&router, OWNER, ("GET", "/serpdad/"), None).await;
    assert_eq!(read, json!({"connection": null}));
    let (status, json) = send(&router, OWNER, ("POST", "/serpdad/"), None).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{json}");
    let fix = json["fix"].as_str().unwrap_or_default();
    assert!(fix.contains("khalid serve"), "{json}");
}

#[tokio::test]
async fn the_document_lists_the_serpdad_routes() {
    let request = Request::get("/api/openapi.json").body(Body::empty());
    let response = router(Some(store())).oneshot(request.unwrap()).await;
    let bytes = to_bytes(response.unwrap().into_body(), 4 * 1024 * 1024).await;
    let served: Value = serde_json::from_slice(&bytes.unwrap()).unwrap();
    let route = &served["paths"]["/api/admin/v1/serpdad/"];
    for method in ["get", "post", "delete"] {
        assert!(route[method].is_object(), "{method}");
    }
}
