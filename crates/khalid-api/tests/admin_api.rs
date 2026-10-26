//! The Admin API over HTTP: integration keys and staff tokens, scope
//! checks, and reading a post that may still be a draft.

mod common;

use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{HeaderMap, Request, StatusCode};
use chrono::{TimeZone, Utc};
use khalid_api::api;
use khalid_api::keys::{KeyHashing, KeyToken, MemoryKeyStore, Role, Scope, StoredKey};
use khalid_api::post_store::MemoryPostStore;
use khalid_api::resources::{AdminPost, PostStatus};
use khalid_core::Slug;
use serde_json::Value;
use tower::ServiceExt;

/// Keys made up for these tests. They open nothing real.
const WRITER: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const UPLOADER: &str =
    "ki_2222333344445555_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const EDITOR: &str =
    "ks_5555666677778888_0000111122223333444455556666777788889999aaaabbbbccccddddeeeeffff";
const CONTENT: &str =
    "kc_0123456789abcdef_00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";

#[cfg(test)]
fn keys() -> MemoryKeyStore {
    let hashing = KeyHashing::fast_for_tests();
    let token = |text: &str| text.parse::<KeyToken>().unwrap();
    MemoryKeyStore::new(vec![
        StoredKey::for_integration(&token(WRITER), vec![Scope::PostsWrite], &hashing).unwrap(),
        StoredKey::for_integration(&token(UPLOADER), vec![Scope::MediaWrite], &hashing).unwrap(),
        StoredKey::for_staff(&token(EDITOR), Role::Editor, &hashing).unwrap(),
        StoredKey::for_token(&token(CONTENT)),
    ])
}

#[cfg(test)]
fn posts() -> MemoryPostStore {
    let at = Utc.with_ymd_and_hms(2026, 10, 20, 8, 30, 0).unwrap();
    let draft = AdminPost::new(
        "anchor-text-draft",
        Slug::try_new("choosing-anchor-text").unwrap(),
        "Choosing anchor text",
        "Anchor text tells readers where a link goes.",
        at,
    );
    assert_eq!(draft.status(), PostStatus::Draft);
    MemoryPostStore::new(vec![draft])
}

#[cfg(test)]
async fn get(uri: &str, authorization: Option<&str>) -> (StatusCode, HeaderMap, Value) {
    let (router, _) = api::router(
        Arc::new(common::store()),
        Arc::new(posts()),
        Arc::new(keys()),
    );
    let mut request = Request::get(uri);
    if let Some(value) = authorization {
        request = request.header("authorization", value);
    }
    let response = router
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let (parts, body) = response.into_parts();
    let body = to_bytes(body, 1024 * 1024).await.unwrap();
    (
        parts.status,
        parts.headers,
        serde_json::from_slice(&body).unwrap(),
    )
}

#[tokio::test]
async fn a_key_with_posts_write_reads_a_draft() {
    for key in [WRITER, EDITOR] {
        let (status, _, json) = get(
            "/api/admin/v1/posts/anchor-text-draft/",
            Some(&format!("Khalid {key}")),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{key}");
        let post = &json["posts"][0];
        assert_eq!(post["status"], "draft");
        assert_eq!(
            post["markdown"],
            "Anchor text tells readers where a link goes."
        );
        assert_eq!(post["published_at"], Value::Null);
        assert_eq!(post["updated_at"], "2026-10-20T08:30:00Z");
    }
}

#[tokio::test]
async fn no_key_or_a_content_key_is_a_401() {
    let content = format!("Khalid {CONTENT}");
    let wrong_scheme = format!("Khalid-Content {CONTENT}");
    for authorization in [None, Some(content.as_str()), Some(wrong_scheme.as_str())] {
        let (status, headers, json) =
            get("/api/admin/v1/posts/anchor-text-draft/", authorization).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{authorization:?}");
        assert_eq!(headers["www-authenticate"], "Khalid");
        assert_eq!(json["type"], "https://khalid.dev/problems/unauthorized");
    }
}

#[tokio::test]
async fn a_key_without_the_scope_is_a_403_naming_it() {
    let (status, _, json) = get(
        "/api/admin/v1/posts/anchor-text-draft/",
        Some(&format!("Khalid {UPLOADER}")),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(json["type"], "https://khalid.dev/problems/forbidden");
    assert_eq!(json["rule"], "posts:write");
}

#[tokio::test]
async fn a_missing_post_is_a_404() {
    let (status, _, json) = get(
        "/api/admin/v1/posts/no-such-post/",
        Some(&format!("Khalid {WRITER}")),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(json["instance"], "/api/admin/v1/posts/no-such-post/");
}

#[tokio::test]
async fn both_apis_share_one_document() {
    let (status, _, json) = get("/api/openapi.json", None).await;
    assert_eq!(status, StatusCode::OK);
    let read = &json["paths"]["/api/admin/v1/posts/{id}/"]["get"];
    assert_eq!(read["security"][0]["admin_key"], Value::Array(Vec::new()));
    assert!(json["paths"]["/api/content/v1/posts/"].is_object());
    assert_eq!(
        json["components"]["securitySchemes"]["admin_key"]["in"],
        "header"
    );
}
