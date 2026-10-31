//! The visitor counts through the Admin API (KH-GRO-4): page views per
//! day, per source and per page, against the period before. Needs
//! `settings:write`.

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

/// The API on 28 October, with search and `ChatGPT` readers in the last
/// three days and one search reader the three days before.
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
        Views::new(date(24), POST, ReferrerGroup::Search, 1),
        Views::new(date(26), POST, ReferrerGroup::Search, 4),
        Views::new(date(28), POST, ReferrerGroup::Ai(AiTool::ChatGpt), 2),
        Views::new(date(28), "/", ReferrerGroup::Internal, 1),
    ]);
    let now = Utc.with_ymd_and_hms(2026, 10, 28, 15, 0, 0).unwrap();
    let analytics = AnalyticsDesk::new(Arc::new(views), Arc::new(FixedClock::new(now)));
    let door = PublishDoor::new(Arc::new(MemoryPostStore::default()), Arc::new(Silent));
    let content = Arc::new(common::store());
    let (router, _) =
        api::router_with_analytics(content, Arc::new(door), Arc::new(keys), analytics);
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
async fn the_counts_come_per_day_source_and_page() {
    let (status, json) = get(SETTINGS, "/analytics/visitors/?days=3").await;
    assert_eq!(status, StatusCode::OK, "{json}");
    let source = |key, views, before| json!({"source": key, "views": views, "before": before});
    assert_eq!(
        json,
        json!({
            "first": "2026-10-26", "last": "2026-10-28", "days": 3,
            "views": 7, "before": 1, "arrivals": 6,
            "sources": [
                source("direct", 0, 0),
                source("search", 4, 1),
                source("social", 0, 0),
                source("ai", 2, 0),
                source("other", 0, 0),
                source("internal", 1, 0),
            ],
            "per_day": [
                {"day": "2026-10-26", "views": 4},
                {"day": "2026-10-27", "views": 0},
                {"day": "2026-10-28", "views": 3},
            ],
            "pages": [{"path": POST, "views": 6}, {"path": "/", "views": 1}],
        })
    );
}

#[tokio::test]
async fn four_weeks_are_counted_when_no_days_are_asked_for() {
    let (status, json) = get(SETTINGS, "/analytics/visitors/").await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["days"], 28);
    assert_eq!(json["per_day"].as_array().map(Vec::len), Some(28));
    assert_eq!(json["views"], 8);
}

#[tokio::test]
async fn days_out_of_range_are_refused_and_keys_need_settings_write() {
    let (status, json) = get(SETTINGS, "/analytics/visitors/?days=367").await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{json}");
    let (status, json) = get(POSTS_ONLY, "/analytics/visitors/").await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{json}");
}

#[tokio::test]
async fn the_document_lists_the_route() {
    let request = Request::get("/api/openapi.json").body(Body::empty());
    let response = router().oneshot(request.unwrap()).await;
    let bytes = to_bytes(response.unwrap().into_body(), 4 * 1024 * 1024).await;
    let served: Value = serde_json::from_slice(&bytes.unwrap()).unwrap();
    let route = &served["paths"]["/api/admin/v1/analytics/visitors/"]["get"];
    let days = &route["parameters"][0];
    assert_eq!(
        (&days["name"], &days["in"]),
        (&json!("days"), &json!("query"))
    );
    assert_eq!(days["required"], false, "days may be left out");
}
