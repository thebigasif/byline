//! The honest update switch (F2): an edit to a live post moves its
//! updated date only when it says it is a real update, with a note, and
//! changes what the post says.

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

/// A key made up for these tests. It opens nothing real.
const EDITOR: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";

/// The posts' last change, which every edit here is made on.
const LAST_CHANGE: &str = "2026-10-20T08:30:00Z";

/// The live post's body before any edit.
const BODY: &str = "Anchor text tells readers where a link goes.";

#[cfg(test)]
async fn put(id: &str, edit: &Value) -> (StatusCode, Value) {
    let hashing = KeyHashing::fast_for_tests();
    let token = EDITOR.parse::<KeyToken>().unwrap();
    let scopes = vec![Scope::PostsWrite, Scope::PostsPublish];
    let keys = MemoryKeyStore::new(vec![
        StoredKey::for_integration(&token, scopes, &hashing).unwrap(),
    ]);
    let changed = Utc.with_ymd_and_hms(2026, 10, 20, 8, 30, 0).unwrap();
    let post = |id: &str, slug: &str| {
        AdminPost::new(
            id,
            Slug::try_new(slug).unwrap(),
            "Anchor text",
            BODY,
            changed,
        )
    };
    let live = post("live", "anchor-text").published(changed);
    let draft = post("draft", "anchor-text-draft");
    let now = Utc.with_ymd_and_hms(2026, 10, 31, 9, 0, 0).unwrap();
    let posts = MemoryPostStore::new(vec![live, draft]).with_clock(FixedClock::new(now));
    let (router, _) = api::router(Arc::new(common::store()), Arc::new(posts), Arc::new(keys));
    let body = json!({"posts": [edit]});
    let request = Request::put(format!("/api/admin/v1/posts/{id}/"))
        .header("authorization", format!("Khalid {EDITOR}"))
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let response = router.oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn a_real_update_dates_the_post_today_with_its_note() {
    let edit = json!({
        "markdown": "Anchor text tells readers and search engines where a link goes.",
        "real_update": {"note": "Added what search engines read in anchor text."},
        "updated_at": LAST_CHANGE,
    });
    let (status, json) = put("live", &edit).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    let post = &json["posts"][0];
    assert_eq!(post["updated"], "2026-10-31");
    assert_eq!(
        post["updated_note"],
        "Added what search engines read in anchor text."
    );
}

#[tokio::test]
async fn an_edit_without_the_switch_leaves_the_updated_date_alone() {
    let edit = json!({
        "markdown": "Anchor text tells readers where a link will take them.",
        "updated_at": LAST_CHANGE,
    });
    let (status, json) = put("live", &edit).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["posts"][0]["updated"], Value::Null);
    assert_eq!(json["posts"][0]["updated_note"], Value::Null);
}

#[tokio::test]
async fn a_real_update_that_leaves_the_body_as_it_was_is_refused() {
    let note = json!({"note": "Fixed a typo in the title."});
    let edits = [
        json!({"title": "Choosing anchor text", "real_update": note, "updated_at": LAST_CHANGE}),
        json!({"markdown": BODY, "real_update": note, "updated_at": LAST_CHANGE}),
    ];
    for edit in edits {
        let (status, json) = put("live", &edit).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{edit}");
        assert_eq!(json["rule"], "real_update", "{edit}");
        assert_eq!(
            json["detail"],
            "This edit cannot be marked as a real update. A real update changes what the post says, and this edit leaves its text as it was.",
            "{edit}"
        );
        assert_eq!(
            json["fix"], "Change the post's text in the same edit, or leave real_update out.",
            "{edit}"
        );
    }
}

#[tokio::test]
async fn a_real_update_to_a_post_that_is_not_live_is_refused() {
    let edit = json!({
        "markdown": "Anchor text tells readers where a link will take them.",
        "real_update": {"note": "Rewrote the opening."},
        "updated_at": LAST_CHANGE,
    });
    let (status, json) = put("draft", &edit).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{json}");
    assert_eq!(json["rule"], "real_update");
    assert_eq!(
        json["fix"],
        "Publish the post first, or leave real_update out."
    );
}

#[tokio::test]
async fn a_real_update_that_takes_the_post_down_is_refused() {
    let edit = json!({
        "markdown": "Anchor text tells readers where a link will take them.",
        "real_update": {"note": "Rewrote the opening."},
        "status": "draft",
        "updated_at": LAST_CHANGE,
    });
    let (status, json) = put("live", &edit).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{json}");
    assert_eq!(json["rule"], "real_update");
}

#[tokio::test]
async fn a_real_update_needs_a_note_of_one_short_sentence() {
    let long = "Rewrote it. ".repeat(30);
    for note in [json!(""), json!("   "), json!(long), json!(null)] {
        let edit = json!({
            "markdown": "Anchor text tells readers where a link will take them.",
            "real_update": {"note": note},
            "updated_at": LAST_CHANGE,
        });
        let (status, json) = put("live", &edit).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{note}");
        assert_eq!(
            json["type"], "https://khalid.dev/problems/invalid-body",
            "{note}"
        );
    }
}
