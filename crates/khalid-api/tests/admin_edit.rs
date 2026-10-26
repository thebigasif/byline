//! Editing a post through the Admin API, and the 409 a stale edit gets
//! (RULES.md BL-API-7, KH-API-2).

mod common;

use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use chrono::{TimeZone, Utc};
use khalid_api::api;
use khalid_api::keys::{KeyHashing, KeyToken, MemoryKeyStore, Scope, StoredKey};
use khalid_api::post_store::MemoryPostStore;
use khalid_api::resources::AdminPost;
use khalid_core::{FixedClock, Slug};
use serde_json::{Value, json};
use tower::ServiceExt;

/// Keys made up for these tests. They open nothing real.
const WRITER: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const UPLOADER: &str =
    "ki_2222333344445555_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";

/// The draft's last change, and when the edits in these tests happen.
const LAST_CHANGE: &str = "2026-10-20T08:30:00Z";
const NOW: &str = "2026-10-26T09:00:00Z";

#[cfg(test)]
async fn put(id: &str, key: &str, body: &Value) -> (StatusCode, Value) {
    let hashing = KeyHashing::fast_for_tests();
    let token = |text: &str| text.parse::<KeyToken>().unwrap();
    let keys = MemoryKeyStore::new(vec![
        StoredKey::for_integration(&token(WRITER), vec![Scope::PostsWrite], &hashing).unwrap(),
        StoredKey::for_integration(&token(UPLOADER), vec![Scope::MediaWrite], &hashing).unwrap(),
    ]);
    let draft = AdminPost::new(
        "anchor-text-draft",
        Slug::try_new("choosing-anchor-text").unwrap(),
        "Choosing anchor text",
        "Anchor text tells readers where a link goes.",
        Utc.with_ymd_and_hms(2026, 10, 20, 8, 30, 0).unwrap(),
    )
    .with_updated_by("Asif");
    let now = Utc.with_ymd_and_hms(2026, 10, 26, 9, 0, 0).unwrap();
    let posts = MemoryPostStore::new(vec![draft]).with_clock(FixedClock::new(now));
    let (router, _) = api::router(Arc::new(common::store()), Arc::new(posts), Arc::new(keys));
    let request = Request::put(format!("/api/admin/v1/posts/{id}/"))
        .header("authorization", format!("Khalid {key}"))
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let response = router.oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[cfg(test)]
fn edit(updated_at: &str) -> Value {
    json!({"posts": [{"title": "How to choose anchor text", "updated_at": updated_at}]})
}

#[tokio::test]
async fn an_edit_on_the_latest_copy_is_saved() {
    let (status, json) = put("anchor-text-draft", WRITER, &edit(LAST_CHANGE)).await;
    assert_eq!(status, StatusCode::OK);
    let post = &json["posts"][0];
    assert_eq!(post["title"], "How to choose anchor text");
    assert_eq!(
        post["markdown"],
        "Anchor text tells readers where a link goes."
    );
    assert_eq!(post["updated_at"], NOW);
    assert_eq!(post["updated_by"], "1111222233334444");
}

#[tokio::test]
async fn a_stale_edit_is_a_409_naming_who_and_when() {
    let (status, json) = put("anchor-text-draft", WRITER, &edit("2026-10-19T12:00:00Z")).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(json["type"], "https://khalid.dev/problems/conflict");
    assert_eq!(
        json["detail"],
        "Asif changed this post at 2026-10-20 08:30 UTC, after the copy you edited. Saving your copy would undo their change."
    );
    assert_eq!(json["rule"], "updated_at");
}

#[tokio::test]
async fn a_bad_body_is_a_422_that_says_what_is_wrong() {
    let cases = [
        json!({"posts": [{"title": "No updated_at"}]}),
        json!({"posts": []}),
        json!({"posts": [{"updated_at": LAST_CHANGE}, {"updated_at": LAST_CHANGE}]}),
        json!({"posts": [{"updated_at": LAST_CHANGE, "colour": "red"}]}),
        json!({"title": "Not wrapped in posts"}),
    ];
    for body in cases {
        let (status, json) = put("anchor-text-draft", WRITER, &body).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        assert_eq!(
            json["type"], "https://khalid.dev/problems/invalid-body",
            "{body}"
        );
    }
}

#[tokio::test]
async fn a_missing_post_or_scope_is_refused() {
    let (status, _) = put("no-such-post", WRITER, &edit(LAST_CHANGE)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, json) = put("anchor-text-draft", UPLOADER, &edit(LAST_CHANGE)).await;
    assert_eq!(
        (status, json["rule"].clone()),
        (StatusCode::FORBIDDEN, json!("posts:write"))
    );
}
