//! AI visitors per tool through the Admin API (GEO M1): the readers each
//! AI assistant sent, against the period before. Needs `settings:write`.

mod common;

use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use chrono::{NaiveDate, TimeZone, Utc};
use khalid_analytics::{AiTool, ReferrerGroup, Views};
use khalid_api::analytics_desk::{AnalyticsDesk, ViewsInMemory};
use khalid_api::api;
use khalid_api::keys::{KeyHashing, KeyToken, MemoryKeyStore, Scope, StoredKey};
use khalid_api::post_store::MemoryPostStore;
use khalid_api::publish_door::{PublishDoor, Silent};
use khalid_core::FixedClock;
use serde_json::{Value, json};
use tower::ServiceExt;

/// Keys made up for these tests. They open nothing real.
const SETTINGS: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const POSTS_ONLY: &str =
    "ki_5555666677778888_0000111122223333444455556666777788889999aaaabbbbccccddddeeeeffff";

const POST: &str = "/blog/why-internal-links-matter/";

#[cfg(test)]
fn date(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, day).unwrap()
}

/// The API on 28 October, with `ChatGPT` and Perplexity readers this week,
/// Claude readers the week before, and a search reader.
#[cfg(test)]
fn router() -> axum::Router {
    let hashing = KeyHashing::fast_for_tests();
    let stored = |text: &str, scope| {
        let token: KeyToken = text.parse().unwrap();
        StoredKey::for_integration(&token, vec![scope], &hashing).unwrap()
    };
    let keys = MemoryKeyStore::new(vec![
        stored(SETTINGS, Scope::SettingsWrite),
        stored(POSTS_ONLY, Scope::PostsWrite),
    ]);
    let views = ViewsInMemory::new(vec![
        Views::new(date(18), POST, ReferrerGroup::Ai(AiTool::Claude), 2),
        Views::new(date(24), POST, ReferrerGroup::Ai(AiTool::ChatGpt), 5),
        Views::new(date(27), POST, ReferrerGroup::Ai(AiTool::Perplexity), 1),
        Views::new(date(28), POST, ReferrerGroup::Search, 4),
    ]);
    let now = Utc.with_ymd_and_hms(2026, 10, 28, 15, 0, 0).unwrap();
    let analytics = AnalyticsDesk::new(Arc::new(views), Arc::new(FixedClock::new(now)));
    let door = PublishDoor::new(Arc::new(MemoryPostStore::default()), Arc::new(Silent));
    let content = Arc::new(common::store());
    let keys = Arc::new(keys);
    let (router, _) = api::router_with_analytics(content, Arc::new(door), keys, analytics);
    router
}

/// GETs `path` under the Admin API with `key`; answers the status and the
/// body as JSON.
#[cfg(test)]
async fn get(key: &str, path: &str) -> (StatusCode, Value) {
    let request = Request::get(format!("/api/admin/v1{path}"))
        .header("authorization", format!("Khalid {key}"))
        .body(Body::empty())
        .unwrap();
    let response = router().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn each_tool_shows_its_readers_against_the_week_before() {
    let (status, json) = get(SETTINGS, "/analytics/ai-visitors/?days=7").await;
    assert_eq!(status, StatusCode::OK, "{json}");
    let page = |views| json!([{"path": POST, "views": views}]);
    assert_eq!(
        json,
        json!({
            "first": "2026-10-22", "last": "2026-10-28", "days": 7,
            "arrivals": 10, "ai_views": 6,
            "tools": [
                {"tool": "chatgpt", "name": "ChatGPT", "views": 5, "before": 0, "pages": page(5)},
                {"tool": "perplexity", "name": "Perplexity", "views": 1, "before": 0, "pages": page(1)},
                {"tool": "claude", "name": "Claude", "views": 0, "before": 2, "pages": []},
            ]
        })
    );
}

#[tokio::test]
async fn four_weeks_are_counted_when_no_days_are_asked_for() {
    let (status, json) = get(SETTINGS, "/analytics/ai-visitors/").await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(
        (json["first"].as_str(), json["days"].as_u64()),
        (Some("2026-10-01"), Some(28))
    );
    assert_eq!(json["ai_views"], 8);
}

#[tokio::test]
async fn days_out_of_range_are_refused_in_plain_words() {
    for days in ["0", "367", "a-week"] {
        let path = format!("/analytics/ai-visitors/?days={days}");
        let (status, json) = get(SETTINGS, &path).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{days}: {json}");
        let detail = json["detail"].as_str().unwrap_or_default();
        assert!(
            detail.starts_with("The days are not a number from 1 to 366."),
            "{json}"
        );
    }
}

#[tokio::test]
async fn a_key_without_settings_write_is_refused() {
    let (status, json) = get(POSTS_ONLY, "/analytics/ai-visitors/").await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{json}");
}

#[tokio::test]
async fn the_document_lists_the_route() {
    let request = Request::get("/api/openapi.json").body(Body::empty());
    let response = router().oneshot(request.unwrap()).await;
    let bytes = to_bytes(response.unwrap().into_body(), 4 * 1024 * 1024).await;
    let served: Value = serde_json::from_slice(&bytes.unwrap()).unwrap();
    assert!(served["paths"]["/api/admin/v1/analytics/ai-visitors/"]["get"].is_object());
}
