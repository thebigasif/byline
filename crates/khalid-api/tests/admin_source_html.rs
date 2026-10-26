//! `POST /posts/?source=html`: HTML from another tool becomes markdown,
//! sanitised first (`docs/standards/api.md`, RULES.md BL-SEC-1).

mod common;

use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use khalid_api::api;
use khalid_api::keys::{KeyHashing, KeyToken, MemoryKeyStore, Scope, StoredKey};
use khalid_api::post_store::MemoryPostStore;
use serde_json::{Value, json};
use tower::ServiceExt;

/// A key made up for these tests. It opens nothing real.
const WRITER: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";

#[cfg(test)]
async fn create(query: &str, post: &Value) -> (StatusCode, Value) {
    let token: KeyToken = WRITER.parse().unwrap();
    let hashing = KeyHashing::fast_for_tests();
    let writer = StoredKey::for_integration(&token, vec![Scope::PostsWrite], &hashing).unwrap();
    let keys = MemoryKeyStore::new(vec![writer]);
    let posts = MemoryPostStore::default();
    let (router, _) = api::router(Arc::new(common::store()), Arc::new(posts), Arc::new(keys));
    let request = Request::post(format!("/api/admin/v1/posts/{query}"))
        .header("authorization", format!("Khalid {WRITER}"))
        .header("content-type", "application/json")
        .body(Body::from(json!({"posts": [post]}).to_string()))
        .unwrap();
    let response = router.oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn html_becomes_markdown() {
    let html = "<h2>Why links matter</h2><p>Links <strong>help</strong> readers find <a href=\"https://example.com/blog/\">more</a>.</p>";
    let (status, json) = create("?source=html", &json!({"title": "Links", "html": html})).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(
        json["posts"][0]["markdown"],
        "## Why links matter\n\nLinks **help** readers find [more](https://example.com/blog/)."
    );
}

#[tokio::test]
async fn scripts_and_unsafe_links_are_removed_first() {
    let html = "<p onclick=\"steal()\">Read <a href=\"javascript:steal()\">this</a>.</p><script>steal()</script><iframe src=\"https://evil.example/\"></iframe>";
    let (status, json) = create("?source=html", &json!({"title": "Safe", "html": html})).await;
    assert_eq!(status, StatusCode::CREATED);
    let markdown = json["posts"][0]["markdown"].as_str().unwrap();
    for unsafe_part in [
        "steal",
        "javascript",
        "script",
        "iframe",
        "evil.example",
        "onclick",
    ] {
        assert!(
            !markdown.contains(unsafe_part),
            "{unsafe_part} in {markdown:?}"
        );
    }
    assert!(markdown.contains("Read"), "{markdown:?}");
}

#[tokio::test]
async fn the_body_must_match_the_source() {
    let cases = [
        (
            "?source=html",
            json!({"title": "No HTML", "markdown": "Text."}),
        ),
        (
            "?source=html",
            json!({"title": "Both", "html": "<p>x</p>", "markdown": "x"}),
        ),
        (
            "?source=markdown",
            json!({"title": "HTML as markdown", "html": "<p>x</p>"}),
        ),
        ("", json!({"title": "HTML by default", "html": "<p>x</p>"})),
    ];
    for (query, post) in cases {
        let (status, json) = create(query, &post).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{query} {post}");
        assert_eq!(json["type"], "https://khalid.dev/problems/invalid-body");
    }
    let (status, json) = create("?source=pdf", &json!({"title": "PDF", "markdown": "x"})).await;
    assert_eq!(
        (status, json["rule"].clone()),
        (StatusCode::BAD_REQUEST, json!("source"))
    );
}
