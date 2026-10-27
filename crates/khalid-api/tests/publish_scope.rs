//! Publishing needs `posts:publish`, which editors and above hold and
//! contributors and writers do not (audit 2, brief G6). Writing a draft
//! still needs only `posts:write`.

mod common;

use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use chrono::{TimeZone, Utc};
use khalid_api::api;
use khalid_api::keys::{KeyHashing, KeyToken, MemoryKeyStore, Role, Scope, StoredKey};
use khalid_api::post_store::MemoryPostStore;
use khalid_api::resources::AdminPost;
use khalid_core::{FixedClock, Slug};
use serde_json::{Value, json};
use tower::ServiceExt;

/// Keys made up for these tests. They open nothing real.
const CONTRIBUTOR: &str =
    "ks_1111222233334444_0000111122223333444455556666777788889999aaaabbbbccccddddeeeeffff";
const WRITER: &str =
    "ks_2222333344445555_0000111122223333444455556666777788889999aaaabbbbccccddddeeeeffff";
const EDITOR: &str =
    "ks_3333444455556666_0000111122223333444455556666777788889999aaaabbbbccccddddeeeeffff";
const DRAFTER: &str =
    "ki_4444555566667777_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const PUBLISHER: &str =
    "ki_5555666677778888_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";

#[cfg(test)]
fn keys() -> MemoryKeyStore {
    let hashing = KeyHashing::fast_for_tests();
    let token = |text: &str| text.parse::<KeyToken>().unwrap();
    let both = vec![Scope::PostsWrite, Scope::PostsPublish];
    MemoryKeyStore::new(vec![
        StoredKey::for_staff(&token(CONTRIBUTOR), Role::Contributor, &hashing).unwrap(),
        StoredKey::for_staff(&token(WRITER), Role::Writer, &hashing).unwrap(),
        StoredKey::for_staff(&token(EDITOR), Role::Editor, &hashing).unwrap(),
        StoredKey::for_integration(&token(DRAFTER), vec![Scope::PostsWrite], &hashing).unwrap(),
        StoredKey::for_integration(&token(PUBLISHER), both, &hashing).unwrap(),
    ])
}

/// A draft, and a post that has been live since 20 October.
#[cfg(test)]
fn posts() -> MemoryPostStore {
    let at = Utc.with_ymd_and_hms(2026, 10, 20, 8, 30, 0).unwrap();
    let post = |id: &str, slug: &str, title: &str| {
        let body = "Anchor text tells readers where a link goes.";
        AdminPost::new(id, Slug::try_new(slug).unwrap(), title, body, at)
    };
    let draft = post(
        "en.choosing-anchor-text",
        "choosing-anchor-text",
        "Choosing anchor text",
    );
    let live = post(
        "en.why-links-matter",
        "why-links-matter",
        "Why links matter",
    )
    .published(at);
    let now = Utc.with_ymd_and_hms(2026, 10, 26, 9, 0, 0).unwrap();
    MemoryPostStore::new(vec![draft, live]).with_clock(FixedClock::new(now))
}

/// Sends `edit` to post `id` with `key`, and returns the status and body.
#[cfg(test)]
async fn put(key: &str, id: &str, edit: Value) -> (StatusCode, Value) {
    let (router, _) = api::router(
        Arc::new(common::store()),
        Arc::new(posts()),
        Arc::new(keys()),
    );
    let body = json!({ "posts": [edit] });
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
fn publish() -> Value {
    json!({"status": "published", "updated_at": "2026-10-20T08:30:00Z"})
}

#[tokio::test]
async fn a_contributor_publishing_is_a_403_in_plain_words() {
    let (status, json) = put(CONTRIBUTOR, "en.choosing-anchor-text", publish()).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{json}");
    assert_eq!(json["type"], "https://khalid.dev/problems/forbidden");
    assert_eq!(json["rule"], "posts:publish");
    assert_eq!(
        json["detail"],
        "A contributor cannot publish posts. Only editors, admins and the owner publish, so an editor checks each post before readers see it."
    );
    assert_eq!(
        json["fix"],
        "Save your changes as a draft and ask an editor to publish it."
    );
}

#[tokio::test]
async fn a_contributor_or_writer_cannot_schedule_change_or_take_down_a_live_post() {
    let schedule = json!({
        "status": "scheduled",
        "published_at": "2026-11-03T09:00:00Z",
        "updated_at": "2026-10-20T08:30:00Z",
    });
    let retitle = json!({"title": "Why links still matter", "updated_at": "2026-10-20T08:30:00Z"});
    let take_down = json!({"status": "draft", "updated_at": "2026-10-20T08:30:00Z"});
    for key in [CONTRIBUTOR, WRITER] {
        let (status, json) = put(key, "en.choosing-anchor-text", schedule.clone()).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "schedule: {json}");
        let (status, json) = put(key, "en.why-links-matter", retitle.clone()).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "live edit: {json}");
        let (status, json) = put(key, "en.why-links-matter", take_down.clone()).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "take down: {json}");
    }
}

#[tokio::test]
async fn a_contributor_still_edits_a_draft() {
    let retitle = json!({"title": "Picking anchor text", "updated_at": "2026-10-20T08:30:00Z"});
    let (status, json) = put(CONTRIBUTOR, "en.choosing-anchor-text", retitle).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["posts"][0]["status"], "draft");
}

#[tokio::test]
async fn an_editor_publishes() {
    let (status, json) = put(EDITOR, "en.choosing-anchor-text", publish()).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["posts"][0]["status"], "published");
}

#[tokio::test]
async fn an_integration_key_publishes_only_with_posts_publish() {
    let (status, json) = put(DRAFTER, "en.choosing-anchor-text", publish()).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{json}");
    assert_eq!(json["rule"], "posts:publish");
    assert_eq!(
        json["detail"],
        "This key does not have the posts:publish scope. Each key may do only what it was made for, so a leaked key does less harm."
    );
    let (status, json) = put(PUBLISHER, "en.choosing-anchor-text", publish()).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["posts"][0]["status"], "published");
}
