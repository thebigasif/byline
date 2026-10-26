//! Creating a post through the Admin API, once per idempotency key
//! (RULES.md BL-API-7, KH-API-2).

mod common;

use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{HeaderMap, Request, StatusCode};
use chrono::{TimeZone, Utc};
use khalid_api::api;
use khalid_api::keys::{KeyHashing, KeyToken, MemoryKeyStore, Scope, StoredKey};
use khalid_api::post_store::MemoryPostStore;
use khalid_core::FixedClock;
use serde_json::{Value, json};
use tower::ServiceExt;

/// Keys made up for these tests. They open nothing real.
const WRITER: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";

#[cfg(test)]
fn router() -> Router {
    let hashing = KeyHashing::fast_for_tests();
    let token = |text: &str| text.parse::<KeyToken>().unwrap();
    let keys = MemoryKeyStore::new(vec![
        StoredKey::for_integration(&token(WRITER), vec![Scope::PostsWrite], &hashing).unwrap(),
    ]);
    let now = Utc.with_ymd_and_hms(2026, 10, 26, 9, 0, 0).unwrap();
    let posts = MemoryPostStore::default().with_clock(FixedClock::new(now));
    let (router, _) = api::router(Arc::new(common::store()), Arc::new(posts), Arc::new(keys));
    router
}

#[cfg(test)]
async fn post(
    router: &Router,
    key: &str,
    idempotency: Option<&str>,
    body: &Value,
) -> (StatusCode, HeaderMap, Value) {
    let mut request = Request::post("/api/admin/v1/posts/")
        .header("authorization", format!("Khalid {key}"))
        .header("content-type", "application/json");
    if let Some(value) = idempotency {
        request = request.header("idempotency-key", value);
    }
    let response = router
        .clone()
        .oneshot(request.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let (parts, body) = response.into_parts();
    let bytes = to_bytes(body, 1024 * 1024).await.unwrap();
    (
        parts.status,
        parts.headers,
        serde_json::from_slice(&bytes).unwrap(),
    )
}

#[cfg(test)]
fn draft(title: &str) -> Value {
    json!({"posts": [{"title": title, "markdown": "Every link is a promise to the reader."}]})
}

#[tokio::test]
async fn a_new_post_is_a_draft_with_a_slug_from_its_title() {
    let router = router();
    let (status, headers, json) = post(&router, WRITER, None, &draft("Über interne Links")).await;
    assert_eq!(status, StatusCode::CREATED);
    let created = &json["posts"][0];
    assert_eq!(created["slug"], "uber-interne-links");
    assert_eq!(created["status"], "draft");
    assert_eq!(created["updated_at"], "2026-10-26T09:00:00Z");
    assert_eq!(created["updated_by"], "1111222233334444");
    let id = created["id"].as_str().unwrap();
    assert_eq!(headers["location"], format!("/api/admin/v1/posts/{id}/"));
}

#[tokio::test]
async fn a_given_slug_is_kept_and_a_taken_one_is_a_409() {
    let router = router();
    let body =
        json!({"posts": [{"title": "Links", "slug": "why-links-matter", "markdown": "Links."}]});
    let (status, _, json) = post(&router, WRITER, None, &body).await;
    assert_eq!(
        (status, json["posts"][0]["slug"].clone()),
        (StatusCode::CREATED, json!("why-links-matter"))
    );
    let (status, _, json) = post(&router, WRITER, None, &body).await;
    assert_eq!(
        (status, json["rule"].clone()),
        (StatusCode::CONFLICT, json!("slug"))
    );
}

#[tokio::test]
async fn the_same_idempotency_key_creates_one_post() {
    let router = router();
    let (first_status, _, first) = post(
        &router,
        WRITER,
        Some("serpdad-article-381"),
        &draft("Anchor text"),
    )
    .await;
    let (again_status, _, again) = post(
        &router,
        WRITER,
        Some("serpdad-article-381"),
        &draft("Anchor text"),
    )
    .await;
    assert_eq!(
        (first_status, again_status),
        (StatusCode::CREATED, StatusCode::CREATED)
    );
    assert_eq!(first["posts"][0]["id"], again["posts"][0]["id"]);
}

#[tokio::test]
async fn a_bad_body_is_a_422() {
    let router = router();
    for body in [
        json!({"posts": [{"title": "", "markdown": "Empty title."}]}),
        json!({"posts": [{"title": "No body"}]}),
        json!({"posts": [{"title": "Bad slug", "slug": "Not A Slug", "markdown": "x"}]}),
        json!({"posts": [{"title": "HTML without source", "html": "<p>x</p>"}]}),
    ] {
        let (status, _, json) = post(&router, WRITER, None, &body).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        assert_eq!(
            json["type"], "https://khalid.dev/problems/invalid-body",
            "{body}"
        );
    }
}
