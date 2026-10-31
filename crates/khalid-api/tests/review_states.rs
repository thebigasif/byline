//! Who may move a draft between review states through the Admin API: a
//! writer submits, an editor approves (KH-EDT-4, plan section 4.6).

mod common;

use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use chrono::{TimeZone, Utc};
use khalid_api::api;
use khalid_api::keys::{KeyHashing, KeyToken, MemoryKeyStore, Role, Scope, StoredKey};
use khalid_api::post_store::{MemoryPostStore, PostStore};
use khalid_api::resources::AdminPost;
use khalid_core::{FixedClock, Slug};
use serde_json::{Value, json};
use tower::ServiceExt;

/// Staff tokens and an integration key made up for these tests. They open
/// nothing real.
const CONTRIBUTOR: &str =
    "ks_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const WRITER: &str =
    "ks_2222333344445555_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const EDITOR: &str =
    "ks_3333444455556666_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const DRAFTS_ONLY: &str =
    "ki_4444555566667777_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";

/// Sends each `(token, edit)` in turn to the draft, each on the copy the
/// one before saved, and returns the answers.
#[cfg(test)]
async fn edits(all: Vec<(&str, Value)>) -> Vec<(StatusCode, Value)> {
    let hashing = KeyHashing::fast_for_tests();
    let token = |text: &str| text.parse::<KeyToken>().unwrap();
    let staff = |text: &str, role| StoredKey::for_staff(&token(text), role, &hashing).unwrap();
    let keys = vec![
        staff(CONTRIBUTOR, Role::Contributor),
        staff(WRITER, Role::Writer),
        staff(EDITOR, Role::Editor),
        StoredKey::for_integration(&token(DRAFTS_ONLY), vec![Scope::PostsWrite], &hashing).unwrap(),
    ];
    let at = Utc.with_ymd_and_hms(2026, 10, 20, 8, 30, 0).unwrap();
    let slug = Slug::try_new("notes").unwrap();
    let draft = AdminPost::new("draft", slug, "Notes", "Text.", at);
    let now = FixedClock::new(Utc.with_ymd_and_hms(2026, 10, 26, 9, 0, 0).unwrap());
    let posts: Arc<dyn PostStore> = Arc::new(MemoryPostStore::new(vec![draft]).with_clock(now));
    let keys = Arc::new(MemoryKeyStore::new(keys));
    let (router, _) = api::router(Arc::new(common::store()), posts, keys);
    let mut updated_at = json!("2026-10-20T08:30:00Z");
    let mut answers = Vec::new();
    for (token, mut edit) in all {
        edit["updated_at"] = updated_at.clone();
        let request = Request::put("/api/admin/v1/posts/draft/")
            .header("authorization", format!("Khalid {token}"))
            .header("content-type", "application/json")
            .body(Body::from(json!({"posts": [edit]}).to_string()))
            .unwrap();
        let response = router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let json: Value = serde_json::from_slice(&bytes).unwrap();
        if status == StatusCode::OK {
            updated_at = json["posts"][0]["updated_at"].clone();
        }
        answers.push((status, json));
    }
    answers
}

/// An edit that sets the review state to `state`.
fn to(state: &str) -> Value {
    json!({"settings": {"review_state": state}})
}

#[tokio::test]
async fn a_writer_or_contributor_submits_and_withdraws_but_cannot_approve() {
    for token in [CONTRIBUTOR, WRITER, DRAFTS_ONLY] {
        let answers = edits(vec![
            (token, to("in_review")),
            (token, to("draft")),
            (token, to("in_review")),
            (token, to("ready")),
        ])
        .await;
        for (status, json) in &answers[..3] {
            assert_eq!(*status, StatusCode::OK, "{json}");
        }
        let (status, json) = &answers[3];
        assert_eq!(*status, StatusCode::FORBIDDEN, "{json}");
        assert_eq!(json["rule"], "review_state", "{json}");
        let detail = json["detail"].as_str().unwrap();
        assert!(detail.contains("editor"), "{detail}");
    }
}

#[tokio::test]
async fn an_editor_approves_and_sends_back() {
    let answers = edits(vec![
        (WRITER, to("in_review")),
        (EDITOR, to("ready")),
        (EDITOR, to("in_review")),
    ])
    .await;
    for (status, json) in &answers {
        assert_eq!(*status, StatusCode::OK, "{json}");
    }
    assert_eq!(
        answers[1].1["posts"][0]["settings"]["review_state"],
        "ready"
    );
}

#[tokio::test]
async fn a_writer_changes_an_approved_post_only_by_sending_it_back_to_review() {
    let answers = edits(vec![
        (EDITOR, to("ready")),
        (WRITER, json!({"title": "Better notes"})),
        (WRITER, to("draft")),
    ])
    .await;
    let (status, json) = &answers[1];
    assert_eq!(*status, StatusCode::FORBIDDEN, "{json}");
    assert_eq!(json["rule"], "review_state", "{json}");
    assert!(
        json["fix"].as_str().unwrap().contains("in_review"),
        "{json}"
    );
    let send_back = json!({"title": "Better notes", "settings": {"review_state": "in_review"}});
    let answers = edits(vec![(EDITOR, to("ready")), (WRITER, send_back)]).await;
    let (status, json) = &answers[1];
    assert_eq!(*status, StatusCode::OK, "{json}");
    assert_eq!(json["posts"][0]["title"], "Better notes");
    assert_eq!(json["posts"][0]["settings"]["review_state"], "in_review");
}

#[tokio::test]
async fn sending_ready_back_with_a_change_is_still_refused() {
    let answers = edits(vec![
        (EDITOR, to("ready")),
        (
            WRITER,
            json!({"settings": {"review_state": "ready", "featured": true}}),
        ),
    ])
    .await;
    let (status, json) = &answers[1];
    assert_eq!(
        *status,
        StatusCode::FORBIDDEN,
        "a change to a ready post: {json}"
    );
}
