//! The one publish door: an Admin API edit that takes a draft live is
//! announced once, to every receiver the site has, and nothing else is
//! (KH-API-3, KH-SEO-11, CTO answers 2026-10-27 section 4).

mod common;

use std::sync::{Arc, Mutex};

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use chrono::{TimeZone, Utc};
use khalid_api::api;
use khalid_api::keys::{KeyHashing, KeyToken, MemoryKeyStore, Scope, StoredKey};
use khalid_api::post_store::MemoryPostStore;
use khalid_api::publish_door::{Announce, PublishDoor};
use khalid_api::resources::AdminPost;
use khalid_core::{FixedClock, Slug};
use serde_json::{Value, json};
use tower::ServiceExt;

/// A key made up for these tests. It opens nothing real.
const WRITER: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";

/// Writes down the identity of every post it is told went live.
#[derive(Debug, Default)]
struct Recorder(Mutex<Vec<String>>);

#[cfg(test)]
impl Announce for Recorder {
    fn published(&self, post: &AdminPost) {
        self.0.lock().unwrap().push(post.id().to_owned());
    }

    fn unpublished(&self, post: &AdminPost) {
        self.0.lock().unwrap().push(format!("down {}", post.id()));
    }
}

/// The router over one draft, sending to `recorder`, with a key that may
/// write posts.
#[cfg(test)]
fn router(recorder: &Arc<Recorder>) -> axum::Router {
    let draft = AdminPost::new(
        "en.choosing-anchor-text",
        Slug::try_new("choosing-anchor-text").unwrap(),
        "Choosing anchor text",
        "Anchor text tells readers where a link goes.",
        Utc.with_ymd_and_hms(2026, 10, 20, 8, 30, 0).unwrap(),
    );
    let now = Utc.with_ymd_and_hms(2026, 10, 26, 9, 0, 0).unwrap();
    let posts = MemoryPostStore::new(vec![draft]).with_clock(FixedClock::new(now));
    let announce: Arc<dyn Announce> = Arc::<Recorder>::clone(recorder);
    let door = PublishDoor::new(Arc::new(posts), announce);
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
    let (router, _) =
        api::router_with_door(Arc::new(common::store()), Arc::new(door), Arc::new(keys));
    router
}

/// Sends `edit` to the draft and returns the status and the post as saved.
#[cfg(test)]
async fn put(router: &axum::Router, edit: Value) -> (StatusCode, Value) {
    let request = Request::put("/api/admin/v1/posts/en.choosing-anchor-text/")
        .header("authorization", format!("Khalid {WRITER}"))
        .header("content-type", "application/json")
        .body(Body::from(json!({ "posts": [edit] }).to_string()))
        .unwrap();
    let response = router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn a_draft_that_goes_live_is_announced_once() {
    let recorder = Arc::new(Recorder::default());
    let router = router(&recorder);
    let (status, json) = put(
        &router,
        json!({"status": "published", "updated_at": "2026-10-20T08:30:00Z"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(*recorder.0.lock().unwrap(), ["en.choosing-anchor-text"]);

    // Publishing a live post again changes nothing to announce.
    let again = json!({"status": "published", "updated_at": json["posts"][0]["updated_at"]});
    let (status, json) = put(&router, again).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(recorder.0.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn a_live_post_taken_down_is_announced_once() {
    let recorder = Arc::new(Recorder::default());
    let router = router(&recorder);
    let up = json!({"status": "published", "updated_at": "2026-10-20T08:30:00Z"});
    let (_, json) = put(&router, up).await;
    let down = json!({"status": "draft", "updated_at": json["posts"][0]["updated_at"]});
    let (status, json) = put(&router, down).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["posts"][0]["status"], "draft", "{json}");
    assert_eq!(json["posts"][0]["published_at"], Value::Null, "{json}");
    let again = json!({"status": "draft", "updated_at": json["posts"][0]["updated_at"]});
    put(&router, again).await;
    let told = recorder.0.lock().unwrap().clone();
    assert_eq!(
        told,
        ["en.choosing-anchor-text", "down en.choosing-anchor-text"]
    );
}

#[tokio::test]
async fn a_live_post_deleted_is_announced_as_taken_down() {
    let recorder = Arc::new(Recorder::default());
    let router = router(&recorder);
    let up = json!({"status": "published", "updated_at": "2026-10-20T08:30:00Z"});
    let (_, json) = put(&router, up).await;
    let seen = json["posts"][0]["updated_at"].as_str().unwrap();
    let request = Request::delete(format!(
        "/api/admin/v1/posts/en.choosing-anchor-text/?updated_at={seen}"
    ))
    .header("authorization", format!("Khalid {WRITER}"))
    .body(Body::empty())
    .unwrap();
    let response = router.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let told = recorder.0.lock().unwrap().clone();
    assert_eq!(
        told,
        ["en.choosing-anchor-text", "down en.choosing-anchor-text"]
    );
}

#[tokio::test]
async fn an_edit_that_does_not_publish_is_not_announced() {
    let recorder = Arc::new(Recorder::default());
    let router = router(&recorder);
    let edit = json!({"title": "Anchor text, chosen well", "updated_at": "2026-10-20T08:30:00Z"});
    let (status, json) = put(&router, edit).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert!(recorder.0.lock().unwrap().is_empty());
}

#[tokio::test]
async fn a_stale_publish_is_refused_and_not_announced() {
    let recorder = Arc::new(Recorder::default());
    let router = router(&recorder);
    let stale = json!({"status": "published", "updated_at": "2026-10-19T08:30:00Z"});
    let (status, json) = put(&router, stale).await;
    assert_eq!(status, StatusCode::CONFLICT, "{json}");
    assert!(recorder.0.lock().unwrap().is_empty());
}
