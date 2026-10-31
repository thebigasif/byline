//! The calendar through the Admin API: drafts, scheduled posts and
//! published posts by day (KH-EDT-4, plan section 4.6).

mod common;

use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use khalid_api::api;
use khalid_api::keys::{KeyHashing, KeyToken, MemoryKeyStore, Scope, StoredKey};
use khalid_api::post_store::MemoryPostStore;
use khalid_api::publish_door::{PublishDoor, Silent};
use khalid_api::resources::{AdminPost, PostSettings, ReviewState};
use khalid_api::schedule_store::{MemoryScheduleStore, Schedule, ScheduleStore};
use khalid_core::{FixedClock, Slug};
use serde_json::{Value, json};
use tower::ServiceExt;

/// A key made up for these tests. It opens nothing real.
const WRITER: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";

#[cfg(test)]
fn at(month: u32, day: u32, hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, month, day, hour, 0, 0).unwrap()
}

/// Five posts: live in November, live in September, a draft dated for
/// November in review, a draft scheduled in November, and an undated
/// draft.
#[cfg(test)]
async fn calendar(query: &str) -> (StatusCode, Value) {
    let post = |slug: &str| {
        let id = format!("en.{slug}");
        AdminPost::new(
            &id,
            Slug::try_new(slug).unwrap(),
            slug,
            "Text.",
            at(10, 1, 9),
        )
    };
    let mut planned = PostSettings::default();
    planned.date = NaiveDate::from_ymd_opt(2026, 11, 10);
    planned.review_state = ReviewState::InReview;
    let posts = vec![
        post("live").published(at(11, 3, 9)),
        post("old").published(at(9, 1, 9)),
        post("planned").with_settings(planned),
        post("timed"),
        post("idea"),
    ];
    let schedules = Arc::new(MemoryScheduleStore::default());
    let timed = Schedule::default().publishing_at(at(11, 20, 8));
    schedules.set("en.timed", timed).await.unwrap();
    let door = PublishDoor::new(Arc::new(MemoryPostStore::new(posts)), Arc::new(Silent))
        .with_schedules(schedules, FixedClock::new(at(10, 31, 9)));
    let token: KeyToken = WRITER.parse().unwrap();
    let hashing = KeyHashing::fast_for_tests();
    let key = StoredKey::for_integration(&token, vec![Scope::PostsWrite], &hashing).unwrap();
    let keys = Arc::new(MemoryKeyStore::new(vec![key]));
    let (router, _) = api::router_with_door(Arc::new(common::store()), Arc::new(door), keys);
    let request = Request::get(format!("/api/admin/v1/calendar/?{query}"))
        .header("authorization", format!("Khalid {WRITER}"))
        .body(Body::empty())
        .unwrap();
    let response = router.oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn a_month_shows_live_planned_and_scheduled_posts_by_day() {
    let (status, json) = calendar("from=2026-11-01&to=2026-11-30").await;
    assert_eq!(status, StatusCode::OK, "{json}");
    let calendar = &json["calendar"];
    assert_eq!(calendar["from"], "2026-11-01");
    assert_eq!(calendar["to"], "2026-11-30");
    let want = json!([
        {"date": "2026-11-03", "posts": [{"id": "en.live", "title": "live", "status": "published", "review_state": "draft", "at": "2026-11-03T09:00:00Z"}]},
        {"date": "2026-11-10", "posts": [{"id": "en.planned", "title": "planned", "status": "draft", "review_state": "in_review", "at": null}]},
        {"date": "2026-11-20", "posts": [{"id": "en.timed", "title": "timed", "status": "scheduled", "review_state": "draft", "at": "2026-11-20T08:00:00Z"}]},
    ]);
    assert_eq!(calendar["days"], want, "{json}");
    let undated: Vec<&str> = calendar["undated"]
        .as_array()
        .unwrap()
        .iter()
        .map(|post| post["id"].as_str().unwrap())
        .collect();
    assert_eq!(undated, ["en.idea"], "{json}");
}

#[tokio::test]
async fn a_range_backwards_or_over_92_days_is_a_400() {
    for query in [
        "from=2026-11-30&to=2026-11-01",
        "from=2026-01-01&to=2026-12-31",
        "from=2026-11-01",
        "from=November&to=2026-11-30",
    ] {
        let (status, json) = calendar(query).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{query}: {json}");
        assert!(
            json["fix"].as_str().is_some_and(|fix| fix.contains("from")),
            "{json}"
        );
    }
}
