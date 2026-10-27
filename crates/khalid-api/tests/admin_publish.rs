//! Publishing a draft through the Admin API: an edit with
//! `"status": "published"`. The store runs the lint publish gate, and a
//! refusal is a 422 carrying every finding (KH-API-2, KH-LNT-3).

mod common;

use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use chrono::{TimeZone, Utc};
use khalid_api::api;
use khalid_api::content_store::{BoxFuture, Listed};
use khalid_api::keys::{KeyHashing, KeyToken, MemoryKeyStore, Scope, StoredKey};
use khalid_api::post_store::{
    CreatePostError, MemoryPostStore, PostStore, PostStoreError, UpdatePostError,
};
use khalid_api::query::ListQuery;
use khalid_api::resources::{AdminPost, PostDraft, PostEdit, PostRow};
use khalid_core::{FixedClock, Locale, Slug};
use khalid_lint::{LintConfig, LintPost, LintSite, Linter, Verdict};
use serde_json::{Value, json};
use tower::ServiceExt;

/// A key made up for these tests. It opens nothing real.
const WRITER: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";

#[cfg(test)]
fn draft() -> AdminPost {
    AdminPost::new(
        "en.choosing-anchor-text",
        Slug::try_new("choosing-anchor-text").unwrap(),
        "Choosing anchor text",
        "Anchor text tells readers where a link goes.",
        Utc.with_ymd_and_hms(2026, 10, 20, 8, 30, 0).unwrap(),
    )
}

/// Sends `body` as a PUT to the draft through a router over `posts`.
#[cfg(test)]
async fn put(posts: impl PostStore + 'static, body: &Value) -> (StatusCode, Value) {
    let token: KeyToken = WRITER.parse().unwrap();
    let hashing = KeyHashing::fast_for_tests();
    let keys = MemoryKeyStore::new(vec![
        StoredKey::for_integration(
            &token,
            vec![Scope::PostsWrite, Scope::PostsPublish],
            &hashing,
        )
        .unwrap(),
    ]);
    let (router, _) = api::router(Arc::new(common::store()), Arc::new(posts), Arc::new(keys));
    let request = Request::put("/api/admin/v1/posts/en.choosing-anchor-text/")
        .header("authorization", format!("Khalid {WRITER}"))
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let response = router.oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[cfg(test)]
fn publish() -> Value {
    json!({"posts": [{"status": "published", "updated_at": "2026-10-20T08:30:00Z"}]})
}

#[tokio::test]
async fn a_draft_the_gate_passes_goes_live_now() {
    let now = Utc.with_ymd_and_hms(2026, 10, 26, 9, 0, 0).unwrap();
    let posts = MemoryPostStore::new(vec![draft()]).with_clock(FixedClock::new(now));
    let (status, json) = put(posts, &publish()).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    let post = &json["posts"][0];
    assert_eq!(post["status"], "published");
    assert_eq!(post["published_at"], "2026-10-26T09:00:00Z");
    assert_eq!(post["updated_at"], "2026-10-26T09:00:00Z");
}

#[tokio::test]
async fn a_status_that_is_not_one_is_a_422() {
    let posts = MemoryPostStore::new(vec![draft()]);
    let body = json!({"posts": [{"status": "live", "updated_at": "2026-10-20T08:30:00Z"}]});
    let (status, json) = put(posts, &body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{json}");
}

/// A store whose gate refuses every post, with the findings the real
/// linter gives a post with nothing in it.
#[cfg(test)]
struct Refusing;

#[cfg(test)]
impl PostStore for Refusing {
    fn get<'a>(&'a self, _id: &'a str) -> BoxFuture<'a, Result<Option<AdminPost>, PostStoreError>> {
        Box::pin(std::future::ready(Ok(Some(draft()))))
    }

    fn list<'a>(
        &'a self,
        _query: &'a ListQuery,
    ) -> BoxFuture<'a, Result<Listed<PostRow>, PostStoreError>> {
        Box::pin(std::future::ready(Ok(Listed::new(Vec::new(), 0))))
    }

    fn update<'a>(
        &'a self,
        _id: &'a str,
        _edit: PostEdit,
        _by: &'a str,
    ) -> BoxFuture<'a, Result<AdminPost, UpdatePostError>> {
        let today = Utc.with_ymd_and_hms(2026, 10, 26, 9, 0, 0).unwrap();
        let linter = Linter::new(&LintConfig::default(), &FixedClock::new(today)).unwrap();
        let post = LintPost::new(
            Locale::try_new("en").unwrap(),
            Slug::try_new("choosing-anchor-text").unwrap(),
        );
        let Verdict::Refuse { report } = linter.gate(&post, &LintSite::default()) else {
            panic!("a post with nothing in it was let through");
        };
        Box::pin(std::future::ready(Err(UpdatePostError::Refused(report))))
    }

    fn create<'a>(
        &'a self,
        _draft: PostDraft,
        _by: &'a str,
        _idempotency: Option<&'a str>,
    ) -> BoxFuture<'a, Result<AdminPost, CreatePostError>> {
        Box::pin(std::future::ready(Err(CreatePostError::NoSlug)))
    }
}

#[tokio::test]
async fn a_draft_the_gate_refuses_is_a_422_with_every_finding() {
    let (status, json) = put(Refusing, &publish()).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{json}");
    assert_eq!(json["type"], "https://khalid.dev/problems/lint-blocked");
    assert_eq!(json["lint"]["blocks"], true);
    let findings = json["lint"]["findings"].as_array().unwrap();
    let rules: Vec<&str> = findings
        .iter()
        .filter_map(|finding| finding["rule"].as_str())
        .collect();
    assert!(rules.contains(&"no-author"), "{json}");
    assert!(rules.contains(&"no-date"), "{json}");
}

#[tokio::test]
async fn a_new_post_keeps_its_author() {
    let token: KeyToken = WRITER.parse().unwrap();
    let hashing = KeyHashing::fast_for_tests();
    let keys = MemoryKeyStore::new(vec![
        StoredKey::for_integration(
            &token,
            vec![Scope::PostsWrite, Scope::PostsPublish],
            &hashing,
        )
        .unwrap(),
    ]);
    let posts = MemoryPostStore::default();
    let (router, _) = api::router(Arc::new(common::store()), Arc::new(posts), Arc::new(keys));
    let body = json!({"posts": [{"title": "Choosing anchor text", "author": "asif", "markdown": "Words."}]});
    let request = Request::post("/api/admin/v1/posts/")
        .header("authorization", format!("Khalid {WRITER}"))
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let json: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["posts"][0]["author"], "asif");
}
