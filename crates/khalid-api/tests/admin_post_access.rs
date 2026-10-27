//! A post's password, access, template and head code through the Admin
//! API: the password is never sent back, and only owners and admins may
//! change head code (KH-EDT-3, KH-API-2).

mod common;

use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use chrono::{TimeZone, Utc};
use khalid_api::api;
use khalid_api::keys::{KeyHashing, KeyToken, MemoryKeyStore, Role, StoredKey};
use khalid_api::post_store::{MemoryPostStore, PostStore};
use khalid_api::resources::AdminPost;
use khalid_core::{FixedClock, Slug};
use serde_json::{Value, json};
use tower::ServiceExt;

/// Staff tokens made up for these tests. They open nothing real.
const EDITOR: &str =
    "ks_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const ADMIN: &str =
    "ks_5555666677778888_0000111122223333444455556666777788889999aaaabbbbccccddddeeeeffff";

/// A password made up for these tests.
const PASSWORD: &str = "generated-for-this-test";

/// Sends each `(token, settings)` in turn as an edit to the draft, each on
/// the copy the one before saved, and returns the answers and the store.
#[cfg(test)]
async fn edits(all: Vec<(&str, Value)>) -> (Vec<(StatusCode, Value)>, Arc<MemoryPostStore>) {
    let hashing = KeyHashing::fast_for_tests();
    let staff = |text: &str, role| {
        StoredKey::for_staff(&text.parse::<KeyToken>().unwrap(), role, &hashing).unwrap()
    };
    let keys = vec![staff(EDITOR, Role::Editor), staff(ADMIN, Role::Admin)];
    let at = Utc.with_ymd_and_hms(2026, 10, 20, 8, 30, 0).unwrap();
    let draft = AdminPost::new(
        "draft",
        Slug::try_new("notes").unwrap(),
        "Notes",
        "Text.",
        at,
    );
    let now = FixedClock::new(Utc.with_ymd_and_hms(2026, 10, 26, 9, 0, 0).unwrap());
    let posts = Arc::new(MemoryPostStore::new(vec![draft]).with_clock(now));
    let keys = Arc::new(MemoryKeyStore::new(keys));
    let shared: Arc<dyn PostStore> = Arc::<MemoryPostStore>::clone(&posts);
    let (router, _) = api::router(Arc::new(common::store()), shared, keys);
    let mut updated_at = json!("2026-10-20T08:30:00Z");
    let mut answers = Vec::new();
    for (token, settings) in all {
        let edit = json!({"posts": [{"settings": settings, "updated_at": updated_at}]});
        let request = Request::put("/api/admin/v1/posts/draft/")
            .header("authorization", format!("Khalid {token}"))
            .header("content-type", "application/json")
            .body(Body::from(edit.to_string()))
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
    (answers, posts)
}

#[tokio::test]
async fn a_password_is_kept_as_a_hash_and_never_sent_back() {
    let set = (EDITOR, json!({"password": PASSWORD, "access": "members"}));
    let (answers, posts) = edits(vec![set]).await;
    let (status, json) = &answers[0];
    assert_eq!(*status, StatusCode::OK, "{json}");
    let settings = &json["posts"][0]["settings"];
    assert_eq!(settings["password_protected"], true);
    assert_eq!(settings["access"], "members");
    assert!(!json.to_string().contains(PASSWORD), "{json}");
    assert!(!json.to_string().contains("argon2"), "{json}");
    let saved = posts.get("draft").await.unwrap().unwrap();
    let hash = saved.settings().password.clone().unwrap();
    assert!(khalid_api::post_password::password_matches(&hash, PASSWORD));
    assert!(!khalid_api::post_password::password_matches(
        &hash,
        "something-else"
    ));
}

#[tokio::test]
async fn null_removes_the_password_and_template() {
    let set = (EDITOR, json!({"password": PASSWORD, "template": "wide"}));
    let clear = (EDITOR, json!({"password": null, "template": null}));
    let (answers, _) = edits(vec![set, clear]).await;
    assert_eq!(answers[0].1["posts"][0]["settings"]["template"], "wide");
    let (status, json) = &answers[1];
    assert_eq!(*status, StatusCode::OK, "{json}");
    let settings = &json["posts"][0]["settings"];
    assert_eq!(settings["password_protected"], false);
    assert_eq!(settings["template"], Value::Null);
}

#[tokio::test]
async fn only_an_owner_or_admin_may_change_head_code() {
    let code = "<script async src='https://example.com/a.js'></script>";
    let by_editor = (EDITOR, json!({"head_code": code}));
    let by_admin = (ADMIN, json!({"head_code": code}));
    let clear_by_editor = (EDITOR, json!({"head_code": null}));
    let same_by_editor = (EDITOR, json!({"head_code": code, "featured": true}));
    let all = vec![by_editor, by_admin, clear_by_editor, same_by_editor];
    let (answers, _) = edits(all).await;
    let (status, json) = &answers[3];
    assert_eq!(
        *status,
        StatusCode::OK,
        "the same head code is no change: {json}"
    );
    let (status, json) = &answers[0];
    assert_eq!(*status, StatusCode::FORBIDDEN, "{json}");
    assert_eq!(json["rule"], "head_code");
    let (status, json) = &answers[1];
    assert_eq!(*status, StatusCode::OK, "{json}");
    let saved = &json["posts"][0]["settings"]["head_code"];
    assert_eq!(
        saved,
        "<script async src=\"https://example.com/a.js\"></script>"
    );
    assert_eq!(
        answers[2].0,
        StatusCode::FORBIDDEN,
        "clearing is a change too"
    );
}

#[tokio::test]
async fn head_code_that_may_not_be_in_a_head_is_a_422() {
    let cases = ["<div>hi</div>", "<base href=/x/>", "<script>open()"];
    let all = cases.map(|code| (ADMIN, json!({"head_code": code})));
    let (answers, _) = edits(all.to_vec()).await;
    for (status, json) in answers {
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{json}");
        let detail = json["detail"].as_str().unwrap();
        assert!(detail.contains("head code"), "{json}");
    }
}
