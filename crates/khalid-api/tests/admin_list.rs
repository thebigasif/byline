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

/// Keys made up for these tests. They open nothing real.
const WRITER: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const UPLOADER: &str =
    "ki_2222333344445555_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";

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
        StoredKey::for_integration(&token(UPLOADER), vec![Scope::MediaWrite], &hashing).unwrap(),
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
    let rows = json["posts"].as_array().unwrap();
    rows.iter().map(|row| row["id"].as_str().unwrap()).collect()
}

#[tokio::test]
async fn the_list_holds_every_post_with_its_lint_and_index_status() {
    let (status, json) = list("?order=id", WRITER).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(ids(&json), ["live-post", "other-draft", "weak-draft"]);
    let weak = &json["posts"][2];
    assert_eq!(weak["gate"], "refuses");
    assert_eq!(weak["lint"], json!({"blocks": 2, "warnings": 1}));
    assert_eq!(weak["review_state"], "in_review");
    assert_eq!(weak["index_status"], "not_live");
    assert_eq!(json["posts"][0]["index_status"], "indexable");
    assert!(weak.get("markdown").is_none(), "no body in a row");
    assert_eq!(json["meta"]["pagination"]["total"], 3);
}

#[tokio::test]
async fn filters_order_and_pages_use_the_shared_grammar() {
    let cases = [
        ("?filter=gate:refuses", vec!["weak-draft"]),
        ("?filter=review_state:in_review", vec!["weak-draft"]),
        (
            "?filter=status:draft&order=updated_at%20desc",
            vec!["other-draft", "weak-draft"],
        ),
        ("?filter=index_status:indexable", vec!["live-post"]),
        ("?order=id&limit=2&page=2", vec!["weak-draft"]),
    ];
    for (query, want) in cases {
        let (status, json) = list(query, WRITER).await;
        assert_eq!(status, StatusCode::OK, "{query}: {json}");
        assert_eq!(ids(&json), want, "{query}");
    }
    let (_, json) = list("?order=id&limit=2&page=2", WRITER).await;
    assert_eq!(json["meta"]["pagination"]["pages"], 2);
}

#[tokio::test]
async fn a_list_that_cannot_be_answered_is_a_400_naming_the_parameter() {
    let cases = [
        ("?filter=markdown:~%27link%27", "filter"),
        ("?filter=lint.blocks:%3E0", "filter"),
        ("?order=lint.warnings%20desc", "order"),
        ("?limit=all", "limit"),
        ("?limit=500", "limit"),
        ("?formats=html", "query"),
        ("?include=authors", "query"),
    ];
    for (query, rule) in cases {
        let (status, json) = list(query, WRITER).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{query}: {json}");
        assert_eq!(json["rule"], rule, "{query}: {json}");
    }
}

#[tokio::test]
async fn a_key_without_posts_write_cannot_list() {
    let (status, json) = list("", UPLOADER).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{json}");
}
