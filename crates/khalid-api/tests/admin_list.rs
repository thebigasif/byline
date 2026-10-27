//! Listing posts through the Admin API for the admin's posts table, with
//! the shared query grammar (BL-API-5, KH-API-2).

mod common;

use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use chrono::{TimeZone, Utc};
use khalid_api::api;
use khalid_api::keys::{KeyHashing, KeyToken, MemoryKeyStore, Scope, StoredKey};
use khalid_api::post_store::MemoryPostStore;
use khalid_api::resources::{AdminPost, LintCounts, PostSettings, ReviewState};
use khalid_core::Slug;
use serde_json::{Value, json};
use tower::ServiceExt;

/// A key made up for these tests. It opens nothing real.
const WRITER: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";

/// Three posts: a weak draft in review, a draft, and a live post.
#[cfg(test)]
fn posts() -> MemoryPostStore {
    let at = |day| Utc.with_ymd_and_hms(2026, 10, day, 9, 0, 0).unwrap();
    let post = |slug: &str, day| {
        AdminPost::new(slug, Slug::try_new(slug).unwrap(), slug, "Text.", at(day))
    };
    let mut review = PostSettings::default();
    review.review_state = ReviewState::InReview;
    MemoryPostStore::new(vec![
        post("weak-draft", 20).with_settings(review),
        post("other-draft", 21),
        post("live-post", 1).published(at(2)),
    ])
    .with_lint("weak-draft", LintCounts::new(2, 1))
    .with_lint("live-post", LintCounts::new(0, 3))
}

#[cfg(test)]
async fn list(query: &str, key: &str) -> (StatusCode, Value) {
    let hashing = KeyHashing::fast_for_tests();
    let token = |text: &str| text.parse::<KeyToken>().unwrap();
    let keys = MemoryKeyStore::new(vec![
        StoredKey::for_integration(&token(WRITER), vec![Scope::PostsWrite], &hashing).unwrap(),
    ]);
    let (router, _) = api::router(Arc::new(common::store()), Arc::new(posts()), Arc::new(keys));
    let request = Request::get(format!("/api/admin/v1/posts/{query}"))
        .header("authorization", format!("Khalid {key}"))
        .body(Body::empty())
        .unwrap();
    let response = router.oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[cfg(test)]
fn ids(json: &Value) -> Vec<&str> {
    let rows = json["posts"].as_array().unwrap().iter();
    rows.map(|row| row["id"].as_str().unwrap()).collect()
}

#[tokio::test]
async fn the_list_holds_every_post_with_its_lint_and_index_status() {
    let (status, json) = list("?order=id", WRITER).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(ids(&json), ["live-post", "other-draft", "weak-draft"]);
    let weak = &json["posts"][2];
    assert_eq!(weak["gate"], "refuses");
    assert_eq!(weak["lint"], json!({"blocks": 2, "warnings": 1}));
    assert_eq!(weak["index_status"], "not_live");
    assert_eq!(json["meta"]["pagination"]["total"], 3);
}
