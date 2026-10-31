//! Editing a post's settings through the Admin API: every setting in plan
//! section 4.6 the admin's settings panel sends (KH-EDT-3, KH-API-2).

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
const WRITER: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";

/// The draft's last change, which every edit here was made on.
const LAST_CHANGE: &str = "2026-10-20T08:30:00Z";

#[cfg(test)]
fn post(id: &str, slug: &str) -> AdminPost {
    let at = Utc.with_ymd_and_hms(2026, 10, 20, 8, 30, 0).unwrap();
    AdminPost::new(
        id,
        Slug::try_new(slug).unwrap(),
        "Choosing anchor text",
        "Text.",
        at,
    )
}

/// Sends `fields` as an edit to the draft, beside a second post whose slug
/// is `fixing-slow-pages`.
#[cfg(test)]
async fn edit(fields: Value) -> (StatusCode, Value) {
    let mut answers = edits(vec![fields]).await;
    answers.remove(0)
}

/// Sends each of `all` in turn as an edit to the draft, each on the copy
/// the one before it saved.
#[cfg(test)]
async fn edits(all: Vec<Value>) -> Vec<(StatusCode, Value)> {
    let hashing = KeyHashing::fast_for_tests();
    let token = WRITER.parse::<KeyToken>().unwrap();
    let key = StoredKey::for_integration(&token, vec![Scope::PostsWrite], &hashing).unwrap();
    let now = Utc.with_ymd_and_hms(2026, 10, 26, 9, 0, 0).unwrap();
    let posts = vec![
        post("draft", "choosing-anchor-text"),
        post("other", "fixing-slow-pages"),
    ];
    let posts = MemoryPostStore::new(posts).with_clock(FixedClock::new(now));
    let keys = Arc::new(MemoryKeyStore::new(vec![key]));
    let (router, _) = api::router(Arc::new(common::store()), Arc::new(posts), keys);
    let mut updated_at = json!(LAST_CHANGE);
    let mut answers = Vec::new();
    for mut fields in all {
        fields["updated_at"] = updated_at.clone();
        let request = Request::put("/api/admin/v1/posts/draft/")
            .header("authorization", format!("Khalid {WRITER}"))
            .header("content-type", "application/json")
            .body(Body::from(json!({"posts": [fields]}).to_string()))
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

#[cfg(test)]
fn every_setting() -> Value {
    json!({
        "date": "2026-11-02",
        "reviewed": "2026-10-29",
        "co_authors": ["mira", "jonas"],
        "excerpt": "Pick words that say where the link goes.",
        "feature_image": {
            "src": "/media/anchor-text.jpg",
            "alt": "A highlighted link in a paragraph",
            "caption": "Good anchor text names its target."
        },
        "featured": true,
        "review_state": "in_review",
        "canonical": "https://example.com/anchor-text/",
        "robots": "noindex, nofollow",
        "share_cards": {
            "x": {"title": "Anchor text, done right", "description": null, "image": "/media/x.jpg"},
            "facebook": {"title": "Choosing anchor text", "description": "Say where.", "image": null}
        },
        "aliases": ["/2019/05/anchor-text/"],
        "access": "members",
        "template": "wide",
        "head_code": null
    })
}

#[tokio::test]
async fn an_edit_saves_every_setting_and_sends_it_back() {
    let (status, json) = edit(json!({"settings": every_setting()})).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    let mut sent_back = every_setting();
    let unset = json!({"password_protected": false});
    for (name, value) in unset.as_object().unwrap() {
        sent_back[name] = value.clone();
    }
    assert_eq!(json["posts"][0]["settings"], sent_back);
}

#[tokio::test]
async fn null_clears_a_setting_and_a_left_out_one_stays() {
    let first = json!({"settings": every_setting()});
    let clear = json!({"settings": {"canonical": null, "feature_image": null}});
    let answers = edits(vec![first, clear, json!({"title": "Anchor text"})]).await;
    for (status, json) in &answers {
        assert_eq!(*status, StatusCode::OK, "{json}");
    }
    let saved = &answers[2].1["posts"][0]["settings"];
    assert_eq!(saved["canonical"], Value::Null);
    assert_eq!(saved["feature_image"], Value::Null);
    assert_eq!(saved["featured"], true);
    assert_eq!(saved["aliases"], json!(["/2019/05/anchor-text/"]));
}

#[tokio::test]
async fn an_edit_moves_the_slug_and_sets_then_clears_the_author() {
    let first = json!({"slug": "how-to-choose-anchor-text", "author": "asif"});
    let answers = edits(vec![first, json!({"author": null}), json!({})]).await;
    let saved = &answers[0].1["posts"][0];
    assert_eq!(saved["slug"], "how-to-choose-anchor-text", "{saved}");
    assert_eq!(saved["author"], "asif");
    let cleared = &answers[1].1["posts"][0];
    assert_eq!(cleared["author"], Value::Null, "{cleared}");
    let kept = &answers[2].1["posts"][0];
    assert_eq!(kept["slug"], "how-to-choose-anchor-text", "{kept}");
}

#[tokio::test]
async fn a_slug_another_post_has_is_a_409_and_its_own_is_kept() {
    let (status, json) = edit(json!({"slug": "fixing-slow-pages"})).await;
    assert_eq!(status, StatusCode::CONFLICT, "{json}");
    assert_eq!(json["rule"], "slug");
    let fix = json["fix"].as_str().unwrap();
    assert!(fix.contains("keep the one it has"), "{json}");
    let (status, json) = edit(json!({"slug": "choosing-anchor-text"})).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "a post may send its own slug: {json}"
    );
    let (status, json) = edit(json!({"slug": "Not A Slug"})).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{json}");
}

#[tokio::test]
async fn settings_that_cannot_be_right_are_a_422() {
    let cases = [
        json!({"feature_image": {"src": "/media/a.jpg", "alt": " "}}),
        json!({"feature_image": {"src": "", "alt": "A link"}}),
        json!({"robots": "index, noindex"}),
        json!({"canonical": "/relative/"}),
        json!({"aliases": ["2019/old/"]}),
        json!({"co_authors": ["Not A Slug"]}),
        json!({"review_state": "published"}),
        json!({"date": "2026-02-30"}),
        json!({"share_cards": {"x": {"title": ""}}}),
        json!({"password": "short"}),
        json!({"password_protected": true}),
        json!({"access": "paid"}),
        json!({"template": "../base"}),
    ];
    for settings in cases {
        let (status, json) = edit(json!({"settings": settings})).await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{settings}: {json}"
        );
        let detail = json["detail"].as_str().unwrap();
        assert!(detail.contains("could not be read"), "{json}");
    }
}
