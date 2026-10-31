//! Read but never cited through the Admin API (GEO M2): the pages AI bots
//! read that no AI assistant sent a reader to. Needs `settings:write`.

mod common;

use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use khalid_analytics::{AiTool, CrawlerKind, Crawls, ReferrerGroup, Views};
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

const QUIET: &str = "/blog/quiet-post/";
const CITED: &str = "/blog/cited-post/";

#[cfg(test)]
fn at(day: u32, hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, day, hour, 0, 0).unwrap()
}

#[cfg(test)]
fn read(day: u32, path: &str, crawler: &str, kind: CrawlerKind) -> Crawls {
    Crawls {
        day: at(day, 0).date_naive(),
        path: path.to_owned(),
        crawler: crawler.to_owned(),
        kind,
        hits: 2,
        last_seen: at(day, 9),
        last_status: 200,
    }
}

/// The API on 28 October. AI bots read both posts this week; only one had
/// a reader from an AI assistant. Googlebot is not an AI bot.
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
    let day = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
    let views = vec![Views::new(
        day,
        CITED,
        ReferrerGroup::Ai(AiTool::ChatGpt),
        1,
    )];
    let crawls = vec![
        read(26, QUIET, "GPTBot", CrawlerKind::Ai),
        read(27, QUIET, "ClaudeBot", CrawlerKind::Ai),
        read(27, CITED, "GPTBot", CrawlerKind::Ai),
        read(27, "/blog/searched/", "Googlebot", CrawlerKind::Search),
    ];
    let store = ViewsInMemory::new(views).with_crawls(crawls);
    let now = at(28, 15);
    let analytics = AnalyticsDesk::new(Arc::new(store), Arc::new(FixedClock::new(now)));
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
async fn a_page_ai_bots_read_with_no_ai_reader_is_listed() {
    let (status, json) = get(SETTINGS, "/analytics/uncited/?days=7").await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(
        json,
        json!({
            "first": "2026-10-22", "last": "2026-10-28", "days": 7,
            "pages": [{
                "path": QUIET, "reads": 4, "bots": ["ClaudeBot", "GPTBot"],
                "last_read": "2026-10-27T09:00:00Z",
            }],
        })
    );
}

#[tokio::test]
async fn reads_before_the_period_are_left_out_and_thirty_days_is_the_default() {
    let (_, today) = get(SETTINGS, "/analytics/uncited/?days=1").await;
    assert_eq!(today["pages"], json!([]));
    let (status, json) = get(SETTINGS, "/analytics/uncited/").await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(
        (json["days"].as_u64(), json["first"].as_str()),
        (Some(30), Some("2026-09-29"))
    );
}

#[tokio::test]
async fn days_past_what_the_crawl_log_keeps_are_refused_in_plain_words() {
    for days in ["0", "91", "a-month"] {
        let (status, json) = get(SETTINGS, &format!("/analytics/uncited/?days={days}")).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{days}: {json}");
        let detail = json["detail"].as_str().unwrap_or_default();
        assert!(
            detail.starts_with("The days are not a number from 1 to 90."),
            "{json}"
        );
    }
    let (status, _) = get(POSTS_ONLY, "/analytics/uncited/").await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn the_document_lists_the_route() {
    let request = Request::get("/api/openapi.json").body(Body::empty());
    let response = router().oneshot(request.unwrap()).await;
    let bytes = to_bytes(response.unwrap().into_body(), 4 * 1024 * 1024).await;
    let served: Value = serde_json::from_slice(&bytes.unwrap()).unwrap();
    assert!(served["paths"]["/api/admin/v1/analytics/uncited/"]["get"].is_object());
}
