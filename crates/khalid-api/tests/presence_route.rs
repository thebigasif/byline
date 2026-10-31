//! `/api/admin/v1/posts/{id}/presence/` tells the editor who has a post
//! open and who holds its lock (plan 4.6 and 5.12, KH-EDT-2).

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
use khalid_api::schedule_store::MemoryScheduleStore;
use khalid_core::{FixedClock, Slug};
use serde_json::{Value, json};
use tower::ServiceExt;

/// Keys made up for these tests. They open nothing real.
const MARIA: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const LENA: &str =
    "ki_9999aaaabbbbcccc_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const READER: &str =
    "ki_5555666677778888_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";

const PRESENCE: &str = "/api/admin/v1/posts/en.choosing-anchor-text/presence/";

/// Tells nobody.
struct Quiet;

impl Announce for Quiet {
    fn published(&self, _post: &AdminPost) {}
}

/// The router over one draft, with the clock stopped.
#[cfg(test)]
fn router() -> axum::Router {
    let draft = AdminPost::new(
        "en.choosing-anchor-text",
        Slug::try_new("choosing-anchor-text").unwrap(),
        "Choosing anchor text",
        "Anchor text tells readers where a link goes.",
        Utc.with_ymd_and_hms(2026, 10, 20, 8, 30, 0).unwrap(),
    );
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 10, 31, 9, 0, 0).unwrap());
    let posts = MemoryPostStore::new(vec![draft]).with_clock(clock);
    let door = PublishDoor::new(Arc::new(posts), Arc::new(Quiet))
        .with_schedules(Arc::new(MemoryScheduleStore::default()), clock);
    let hashing = KeyHashing::fast_for_tests();
    let key = |token: &str, scopes| {
        let token: KeyToken = token.parse().unwrap();
        StoredKey::for_integration(&token, scopes, &hashing).unwrap()
    };
    let keys = MemoryKeyStore::new(vec![
        key(MARIA, vec![Scope::PostsWrite]),
        key(LENA, vec![Scope::PostsWrite]),
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

/// Checks in as `key`, asking for `claim`.
#[cfg(test)]
async fn check_in(router: &axum::Router, key: &str, claim: &str) -> Value {
    let body = Some(json!({ "claim": claim }));
    let (status, json) = send(router, ("POST", PRESENCE), key, body).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    json
}

#[tokio::test]
async fn the_first_to_check_in_holds_the_lock_until_someone_takes_over() {
    let router = router();
    let maria = check_in(&router, MARIA, "join").await;
    let holder = maria["holder"]["who"].clone();
    assert_eq!(maria["yours"], json!(true), "{maria}");
    assert_eq!(maria["holder"]["since"], json!("2026-10-31T09:00:00Z"));
    let lena = check_in(&router, LENA, "join").await;
    assert_eq!(lena["yours"], json!(false), "{lena}");
    assert_eq!(lena["holder"]["who"], holder);
    let lena = check_in(&router, LENA, "take_over").await;
    assert_eq!(lena["yours"], json!(true), "{lena}");
    assert_eq!(lena["others"][0]["who"], holder);
    let maria = check_in(&router, MARIA, "join").await;
    assert_eq!(maria["yours"], json!(false), "{maria}");
}

#[tokio::test]
async fn the_presence_route_refuses_a_missing_post_a_reader_and_an_unknown_claim() {
    let router = router();
    let missing = "/api/admin/v1/posts/en.nothing-here/presence/";
    let body = Some(json!({ "claim": "join" }));
    let (status, json) = send(&router, ("POST", missing), MARIA, body.clone()).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{json}");
    assert!(
        json.to_string().contains("No post has this identity."),
        "{json}"
    );
    let (status, json) = send(&router, ("POST", PRESENCE), READER, body).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{json}");
    let odd = Some(json!({ "claim": "steal" }));
    let (status, json) = send(&router, ("POST", PRESENCE), MARIA, odd).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{json}");
}
