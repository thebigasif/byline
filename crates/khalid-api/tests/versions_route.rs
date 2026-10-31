//! `GET /api/admin/v1/posts/{id}/versions/` lists the title and text each
//! edit replaced, the newest first, for the editor's earlier versions
//! (plan 4.6, KH-EDT-2).

mod common;

use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use chrono::{TimeZone, Utc};
use khalid_api::api;
use khalid_api::keys::{KeyHashing, KeyToken, MemoryKeyStore, Scope, StoredKey};
use khalid_api::post_store::MemoryPostStore;
use khalid_api::publish_door::{Announce, PublishDoor};
use khalid_api::resources::AdminPost;
use khalid_api::version_store::MemoryVersionStore;
use khalid_core::{FixedClock, Slug};
use serde_json::{Value, json};
use tower::ServiceExt;

/// A key made up for these tests. It opens nothing real.
const WRITER: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";

/// A key made up for these tests that may only read content.
const READER: &str =
    "ki_5555666677778888_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";

const POST: &str = "/api/admin/v1/posts/en.choosing-anchor-text/";

/// Tells nobody.
struct Quiet;

impl Announce for Quiet {
    fn published(&self, _post: &AdminPost) {}
}

/// The router over one draft, keeping versions in `versions` if given.
#[cfg(test)]
fn router(versions: Option<MemoryVersionStore>) -> axum::Router {
    let draft = AdminPost::new(
        "en.choosing-anchor-text",
        Slug::try_new("choosing-anchor-text").unwrap(),
        "Choosing anchor text",
        "Anchor text tells readers where a link goes.",
        Utc.with_ymd_and_hms(2026, 10, 20, 8, 30, 0).unwrap(),
    );
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap());
    let posts = MemoryPostStore::new(vec![draft]).with_clock(clock);
    let mut door = PublishDoor::new(Arc::new(posts), Arc::new(Quiet));
    if let Some(versions) = versions {
        door = door.with_versions(Arc::new(versions));
    }
    let hashing = KeyHashing::fast_for_tests();
    let key = |token: &str, scopes| {
        let token: KeyToken = token.parse().unwrap();
        StoredKey::for_integration(&token, scopes, &hashing).unwrap()
    };
    let keys = MemoryKeyStore::new(vec![
        key(WRITER, vec![Scope::PostsWrite]),
        key(READER, vec![Scope::ContentRead]),
    ]);
    let content = Arc::new(common::store());
    api::router_with_door(content, Arc::new(door), Arc::new(keys)).0
}

/// Sends `method` to `uri` with `key` and the body `body`, if any.
#[cfg(test)]
async fn send(
    router: &axum::Router,
    (method, uri): (&str, &str),
    key: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let body = body.map_or_else(String::new, |body| body.to_string());
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Khalid {key}"))
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();
    let response = router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn the_versions_route_lists_what_each_edit_replaced() {
    let router = router(Some(MemoryVersionStore::default()));
    let versions = format!("{POST}versions/");
    let (status, json) = send(&router, ("GET", &versions), WRITER, None).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json, json!({ "versions": [] }));
    let edit = json!({ "posts": [{ "markdown": "Anchor text names the page a link opens.",
        "updated_at": "2026-10-20T08:30:00Z" }] });
    let (status, json) = send(&router, ("PUT", POST), WRITER, Some(edit)).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    let (status, json) = send(&router, ("GET", &versions), WRITER, None).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    let kept = json!({ "versions": [{
        "saved_at": "2026-10-20T08:30:00Z",
        "saved_by": null,
        "title": "Choosing anchor text",
        "markdown": "Anchor text tells readers where a link goes.",
    }] });
    assert_eq!(json, kept);
}

#[tokio::test]
async fn the_versions_route_refuses_a_missing_post_a_reader_and_a_server_without_versions() {
    let keeping = router(Some(MemoryVersionStore::default()));
    let missing = "/api/admin/v1/posts/en.nothing-here/versions/";
    let (status, json) = send(&keeping, ("GET", missing), WRITER, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{json}");
    assert!(
        json.to_string().contains("No post has this identity."),
        "{json}"
    );
    let versions = format!("{POST}versions/");
    let (status, json) = send(&keeping, ("GET", &versions), READER, None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{json}");
    let (status, json) = send(&router(None), ("GET", &versions), WRITER, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{json}");
    assert!(
        json.to_string().contains("keeps no earlier versions"),
        "{json}"
    );
}
