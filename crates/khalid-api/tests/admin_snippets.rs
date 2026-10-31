//! The site's snippets through the Admin API: blocks saved once that every
//! post placing them shows (KH-API-2).

use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use chrono::{TimeZone, Utc};
use khalid_api::api::{self, AdminDesks};
use khalid_api::content_store::MemoryContentStore;
use khalid_api::entry_store::{EntryStore, MemoryEntries};
use khalid_api::keys::{KeyHashing, KeyToken, MemoryKeyStore, Scope, StoredKey};
use khalid_api::post_store::MemoryPostStore;
use khalid_api::publish_door::{PublishDoor, Silent};
use khalid_api::resources::AdminSnippet;
use khalid_core::{FixedClock, Locale, Slug};
use serde_json::Value;
use tower::ServiceExt;

/// Keys made up for these tests. They open nothing real.
const WRITER: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const EDITOR: &str =
    "ki_2222333344445555_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";

const SNIPPETS: &str = "/api/admin/v1/snippets/";

/// A site keeping `snippets`, or none at all.
#[cfg(test)]
fn router(snippets: Option<Arc<dyn EntryStore<AdminSnippet>>>) -> Router {
    let hashing = KeyHashing::fast_for_tests();
    let token = |text: &str| text.parse::<KeyToken>().unwrap();
    let key = |text, scopes| StoredKey::for_integration(&token(text), scopes, &hashing).unwrap();
    let keys = MemoryKeyStore::new(vec![
        key(WRITER, vec![Scope::PostsWrite]),
        key(EDITOR, vec![Scope::PostsWrite, Scope::PostsPublish]),
    ]);
    let desks = AdminDesks {
        snippets,
        ..AdminDesks::none()
    };
    let door = PublishDoor::new(Arc::new(MemoryPostStore::default()), Arc::new(Silent));
    let content = Arc::new(MemoryContentStore::new(Vec::new()));
    let (router, _) = api::router_with_desks(content, Arc::new(door), Arc::new(keys), desks);
    router
}

/// A site with one snippet, the affiliate disclosure.
#[cfg(test)]
fn with_disclosure() -> Router {
    let at = Utc.with_ymd_and_hms(2026, 10, 20, 9, 0, 0).unwrap();
    let slug = Slug::try_new("affiliate-disclosure").unwrap();
    let english = Locale::try_new("en").unwrap();
    let disclosure = AdminSnippet::new(
        (english, slug),
        "Affiliate disclosure",
        "Some links pay us.",
        at,
    );
    let now = Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap();
    let snippets = MemoryEntries::new(vec![disclosure]).with_clock(FixedClock::new(now));
    router(Some(Arc::new(snippets)))
}

#[cfg(test)]
async fn send(
    router: &Router,
    method: &str,
    uri: &str,
    key: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Khalid {key}"))
        .header("content-type", "application/json");
    let body = body.map_or_else(Body::empty, |body| Body::from(body.to_string()));
    let response = router
        .clone()
        .oneshot(request.body(body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn snippets_are_listed_with_posts_write() {
    let router = with_disclosure();
    let (status, json) = send(&router, "GET", SNIPPETS, WRITER, None).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(
        json["snippets"][0]["id"], "en.affiliate-disclosure",
        "{json}"
    );
    assert_eq!(
        json["snippets"][0]["markdown"], "Some links pay us.",
        "{json}"
    );
}

#[tokio::test]
async fn a_server_without_snippets_answers_404_naming_why() {
    let (status, json) = send(&router(None), "GET", SNIPPETS, EDITOR, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{json}");
    assert!(
        json["detail"].as_str().unwrap().contains("no snippets"),
        "{json}"
    );
}
