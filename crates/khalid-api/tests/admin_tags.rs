//! The site's tags through the Admin API: listed from their posts, renamed
//! and removed in every one, with the 409 a stale copy gets (KH-API-2).

use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use chrono::{TimeZone, Utc};
use khalid_api::api::{self, AdminDesks};
use khalid_api::content_store::MemoryContentStore;
use khalid_api::keys::{KeyHashing, KeyToken, MemoryKeyStore, Scope, StoredKey};
use khalid_api::post_store::MemoryPostStore;
use khalid_api::publish_door::{PublishDoor, Silent};
use khalid_api::tag_store::{MemoryTags, TagStore, TaggedPost};
use khalid_core::{FixedClock, Locale};
use serde_json::Value;
use tower::ServiceExt;

/// Keys made up for these tests. They open nothing real.
const WRITER: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const EDITOR: &str =
    "ki_2222333344445555_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const READER: &str =
    "ki_3333444455556666_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";

const TAGS: &str = "/api/admin/v1/tags/";

/// A site keeping `tags`, or none at all.
#[cfg(test)]
fn router(tags: Option<Arc<dyn TagStore>>) -> Router {
    let hashing = KeyHashing::fast_for_tests();
    let token = |text: &str| text.parse::<KeyToken>().unwrap();
    let key = |text, scopes| StoredKey::for_integration(&token(text), scopes, &hashing).unwrap();
    let keys = MemoryKeyStore::new(vec![
        key(WRITER, vec![Scope::PostsWrite]),
        key(EDITOR, vec![Scope::PostsWrite, Scope::PostsPublish]),
        key(READER, vec![Scope::ContentRead]),
    ]);
    let desks = AdminDesks {
        tags,
        ..AdminDesks::none()
    };
    let door = PublishDoor::new(Arc::new(MemoryPostStore::default()), Arc::new(Silent));
    let content = Arc::new(MemoryContentStore::new(Vec::new()));
    let (router, _) = api::router_with_desks(content, Arc::new(door), Arc::new(keys), desks);
    router
}

/// Two English posts tagged internal links, the later on 20 October.
#[cfg(test)]
fn tagged() -> Router {
    let post = |tags: &[&str], day| TaggedPost {
        language: Locale::try_new("en").unwrap(),
        tags: tags.iter().map(|&tag| tag.to_owned()).collect(),
        updated_at: Utc.with_ymd_and_hms(2026, 10, day, 9, 0, 0).unwrap(),
    };
    let posts = vec![
        post(&["internal links", "anchor text"], 18),
        post(&["internal links"], 20),
    ];
    let now = Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap();
    router(Some(Arc::new(
        MemoryTags::new(posts).with_clock(FixedClock::new(now)),
    )))
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
    let request = request.body(body).unwrap();
    let response = router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn tags_are_listed_with_their_posts_for_posts_write() {
    let (status, json) = send(&tagged(), "GET", TAGS, WRITER, None).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["tags"][0]["id"], "en.anchor-text", "{json}");
    assert_eq!(json["tags"][1]["id"], "en.internal-links", "{json}");
    assert_eq!(json["tags"][1]["posts"], 2, "{json}");
    assert_eq!(json["tags"][1]["updated_at"], "2026-10-20T09:00:00Z");
}

#[tokio::test]
async fn reading_tags_needs_a_key_with_posts_write_and_a_server_with_tags() {
    let (status, json) = send(&tagged(), "GET", TAGS, READER, None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{json}");
    assert_eq!(json["rule"], "posts:write", "{json}");
    let (status, json) = send(&router(None), "GET", TAGS, EDITOR, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{json}");
    assert!(
        json["detail"].as_str().unwrap().contains("no tags"),
        "{json}"
    );
}
