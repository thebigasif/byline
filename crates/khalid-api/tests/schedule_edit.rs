//! Scheduling through the Admin API: `status: scheduled` with a future
//! `published_at` schedules a draft, `unpublish_at` schedules its removal,
//! `status: draft` cancels, and a schedule that cannot be kept is a 422
//! that changes nothing (KH-API-2, CTO answers 2026-10-27 section 4).

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
use khalid_api::schedule_store::MemoryScheduleStore;
use khalid_core::{FixedClock, Slug};
use serde_json::{Value, json};
use tower::ServiceExt;

/// A key made up for these tests. It opens nothing real.
const WRITER: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";

/// The draft's `updated_at`.
const SAVED: &str = "2026-10-20T08:30:00Z";

/// Writes down each event and the post it was about.
#[derive(Debug, Default)]
struct Recorder(Mutex<Vec<(&'static str, String)>>);

#[cfg(test)]
impl Announce for Recorder {
    fn published(&self, post: &AdminPost) {
        self.0
            .lock()
            .unwrap()
            .push(("published", post.id().to_owned()));
    }

    fn scheduled(&self, post: &AdminPost) {
        self.0
            .lock()
            .unwrap()
            .push(("scheduled", post.id().to_owned()));
    }
}

/// The router over one draft at 9:00 on 27 October 2026, with schedules
/// in `schedules` if given.
#[cfg(test)]
fn router(recorder: &Arc<Recorder>, schedules: Option<MemoryScheduleStore>) -> axum::Router {
    let draft = AdminPost::new(
        "en.choosing-anchor-text",
        Slug::try_new("choosing-anchor-text").unwrap(),
        "Choosing anchor text",
        "Anchor text tells readers where a link goes.",
        Utc.with_ymd_and_hms(2026, 10, 20, 8, 30, 0).unwrap(),
    );
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap());
    let posts = MemoryPostStore::new(vec![draft]).with_clock(clock);
    let announce: Arc<dyn Announce> = Arc::<Recorder>::clone(recorder);
    let mut door = PublishDoor::new(Arc::new(posts), announce);
    if let Some(schedules) = schedules {
        door = door.with_schedules(Arc::new(schedules), clock);
    }
    let token: KeyToken = WRITER.parse().unwrap();
    let hashing = KeyHashing::fast_for_tests();
    let keys = MemoryKeyStore::new(vec![
        StoredKey::for_integration(&token, vec![Scope::PostsWrite], &hashing).unwrap(),
    ]);
    let content = Arc::new(common::store());
    api::router_with_door(content, Arc::new(door), Arc::new(keys)).0
}

/// Sends `method` to the draft with the edit `edit`, if any.
#[cfg(test)]
async fn send(router: &axum::Router, method: &str, edit: Option<Value>) -> (StatusCode, Value) {
    let body = edit.map_or_else(String::new, |edit| json!({ "posts": [edit] }).to_string());
    let request = Request::builder()
        .method(method)
        .uri("/api/admin/v1/posts/en.choosing-anchor-text/")
        .header("authorization", format!("Khalid {WRITER}"))
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();
    let response = router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn a_draft_scheduled_for_later_shows_as_scheduled_and_is_announced() {
    let recorder = Arc::new(Recorder::default());
    let router = router(&recorder, Some(MemoryScheduleStore::default()));
    let edit = json!({"status": "scheduled", "published_at": "2026-11-03T09:00:00+01:00",
        "unpublish_at": "2026-12-01T00:00:00Z", "updated_at": SAVED});
    let (status, json) = send(&router, "PUT", Some(edit)).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    let (status, json) = send(&router, "GET", None).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    let post = &json["posts"][0];
    assert_eq!(post["status"], "scheduled", "{json}");
    assert_eq!(post["published_at"], "2026-11-03T08:00:00Z");
    assert_eq!(post["unpublish_at"], "2026-12-01T00:00:00Z");
    let told = recorder.0.lock().unwrap().clone();
    assert_eq!(told, [("scheduled", "en.choosing-anchor-text".to_owned())]);
}

#[tokio::test]
async fn status_draft_takes_a_post_off_its_schedule() {
    let recorder = Arc::new(Recorder::default());
    let router = router(&recorder, Some(MemoryScheduleStore::default()));
    let edit = json!({"status": "scheduled", "published_at": "2026-11-03T08:00:00Z",
        "updated_at": SAVED});
    let (_, json) = send(&router, "PUT", Some(edit)).await;
    let cancel = json!({"status": "draft", "updated_at": json["posts"][0]["updated_at"]});
    let (status, json) = send(&router, "PUT", Some(cancel)).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["posts"][0]["status"], "draft", "{json}");
    assert_eq!(json["posts"][0]["published_at"], Value::Null, "{json}");
}

#[tokio::test]
async fn a_schedule_that_cannot_be_kept_is_a_422_that_changes_nothing() {
    let refused = [
        json!({"status": "scheduled", "published_at": "2026-10-27T08:00:00Z", "updated_at": SAVED}),
        json!({"status": "scheduled", "updated_at": SAVED}),
        json!({"published_at": "2026-11-03T08:00:00Z", "updated_at": SAVED}),
        json!({"status": "scheduled", "published_at": "2026-11-03T08:00:00Z",
            "unpublish_at": "2026-11-02T08:00:00Z", "updated_at": SAVED}),
        json!({"unpublish_at": "2026-10-26T08:00:00Z", "updated_at": SAVED}),
    ];
    let recorder = Arc::new(Recorder::default());
    let router = router(&recorder, Some(MemoryScheduleStore::default()));
    for edit in refused {
        let (status, json) = send(&router, "PUT", Some(edit.clone())).await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{edit} gave {json}"
        );
        assert_eq!(json["rule"], "published_at", "{json}");
    }
    let (_, json) = send(&router, "GET", None).await;
    assert_eq!(json["posts"][0]["status"], "draft", "{json}");
    assert_eq!(json["posts"][0]["updated_at"], SAVED, "{json}");
    assert!(recorder.0.lock().unwrap().is_empty());
}

#[tokio::test]
async fn a_site_that_keeps_no_schedules_refuses_one() {
    let recorder = Arc::new(Recorder::default());
    let router = router(&recorder, None);
    let edit = json!({"status": "scheduled", "published_at": "2026-11-03T08:00:00Z",
        "updated_at": SAVED});
    let (status, json) = send(&router, "PUT", Some(edit)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{json}");
}
