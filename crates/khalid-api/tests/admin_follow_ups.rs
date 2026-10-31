//! Search Console follow-ups through the Admin API (GEO M3): the long
//! searches that showed each post, and the heading that answers each.
//! Needs `settings:write`.

mod common;

use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use chrono::NaiveDate;
use khalid_api::api;
use khalid_api::content_store::BoxFuture;
use khalid_api::follow_ups::{FollowUp, FollowUpError, FollowUpFinder, FollowUpPost, FollowUps};
use khalid_api::keys::{KeyHashing, KeyToken, MemoryKeyStore, Scope, StoredKey};
use khalid_api::post_store::MemoryPostStore;
use khalid_api::publish_door::{PublishDoor, Silent};
use khalid_core::UserMessage;
use serde_json::{Value, json};
use tower::ServiceExt;

/// Keys made up for these tests. They open nothing real.
const SETTINGS: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const POSTS_ONLY: &str =
    "ki_5555666677778888_0000111122223333444455556666777788889999aaaabbbbccccddddeeeeffff";

/// What the finder in a test answers.
#[derive(Clone, Copy)]
enum Answer {
    Found,
    NotConnected,
    GoogleRefused,
}

struct Finder(Answer);

impl FollowUpFinder for Finder {
    fn follow_ups(&self) -> BoxFuture<'_, Result<FollowUps, FollowUpError>> {
        let answer = match self.0 {
            Answer::Found => Ok(found()),
            Answer::NotConnected => Err(FollowUpError::NotConnected(UserMessage::new(
                "This site is not connected to Google Search Console.",
                "Khalid asks Google nothing until the owner opts in.",
                "Follow the steps in docs/deploy.md.",
            ))),
            Answer::GoogleRefused => Err(FollowUpError::Google(UserMessage::new(
                "Google's quota for these requests is spent for now.",
                "Google allows a set number of requests a day.",
                "Wait until tomorrow.",
            ))),
        };
        Box::pin(async move { answer })
    }
}

/// One post with a long search a heading answers and one none does.
#[cfg(test)]
fn found() -> FollowUps {
    let search = |query: &str, answered_by: Option<&str>| FollowUp {
        query: query.to_owned(),
        clicks: 2.0,
        impressions: 40.0,
        position: 7.5,
        answered_by: answered_by.map(str::to_owned),
    };
    FollowUps {
        property: "sc-domain:example.com".to_owned(),
        first: NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
        last: NaiveDate::from_ymd_opt(2026, 10, 28).unwrap(),
        words: 8,
        posts: vec![FollowUpPost {
            title: "What a sitemap does".to_owned(),
            url: "https://example.com/blog/what-a-sitemap-does/".to_owned(),
            searches: vec![
                search(
                    "how often does google read the sitemap of a small blog",
                    Some("How often Google reads a sitemap"),
                ),
                search("do i need a sitemap if my blog has only ten posts", None),
            ],
        }],
    }
}

#[cfg(test)]
fn router(answer: Answer) -> axum::Router {
    let hashing = KeyHashing::fast_for_tests();
    let stored = |text: &str, scope| {
        let token: KeyToken = text.parse().unwrap();
        StoredKey::for_integration(&token, vec![scope], &hashing).unwrap()
    };
    let keys = MemoryKeyStore::new(vec![
        stored(SETTINGS, Scope::SettingsWrite),
        stored(POSTS_ONLY, Scope::PostsWrite),
    ]);
    let door = PublishDoor::new(Arc::new(MemoryPostStore::default()), Arc::new(Silent));
    let content = Arc::new(common::store());
    let finder = Arc::new(Finder(answer));
    let (router, _) = api::router_with_follow_ups(content, Arc::new(door), Arc::new(keys), finder);
    router
}

/// GETs the follow-ups with `key`; answers the status and the body as JSON.
#[cfg(test)]
async fn get(answer: Answer, key: &str) -> (StatusCode, Value) {
    let request = Request::get("/api/admin/v1/search-console/follow-ups/")
        .header("authorization", format!("Khalid {key}"))
        .body(Body::empty())
        .unwrap();
    let response = router(answer).oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn the_long_searches_come_with_the_heading_that_answers_each() {
    let (status, json) = get(Answer::Found, SETTINGS).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    let search = |query: &str, answered_by: Value| {
        json!({
            "query": query, "clicks": 2.0, "impressions": 40.0, "position": 7.5,
            "answered_by": answered_by,
        })
    };
    assert_eq!(
        json,
        json!({
            "property": "sc-domain:example.com",
            "first": "2026-10-01", "last": "2026-10-28", "words": 8,
            "posts": [{
                "title": "What a sitemap does",
                "url": "https://example.com/blog/what-a-sitemap-does/",
                "searches": [
                    search(
                        "how often does google read the sitemap of a small blog",
                        json!("How often Google reads a sitemap"),
                    ),
                    search("do i need a sitemap if my blog has only ten posts", Value::Null),
                ],
            }],
        })
    );
}

#[tokio::test]
async fn a_site_not_connected_answers_409_with_what_to_do() {
    let (status, json) = get(Answer::NotConnected, SETTINGS).await;
    assert_eq!(status, StatusCode::CONFLICT, "{json}");
    assert_eq!(json["type"], "https://khalid.dev/problems/not-set-up");
    assert_eq!(
        json["detail"],
        "This site is not connected to Google Search Console. \
         Khalid asks Google nothing until the owner opts in."
    );
    assert_eq!(json["fix"], "Follow the steps in docs/deploy.md.");
}

#[tokio::test]
async fn google_refusing_answers_502_in_plain_words() {
    let (status, json) = get(Answer::GoogleRefused, SETTINGS).await;
    assert_eq!(status, StatusCode::BAD_GATEWAY, "{json}");
    assert_eq!(json["type"], "https://khalid.dev/problems/upstream");
    assert_eq!(json["fix"], "Wait until tomorrow.");
}

#[tokio::test]
async fn a_key_without_settings_write_is_refused() {
    let (status, json) = get(Answer::Found, POSTS_ONLY).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{json}");
}

#[tokio::test]
async fn the_document_lists_the_route() {
    let request = Request::get("/api/openapi.json").body(Body::empty());
    let response = router(Answer::Found).oneshot(request.unwrap()).await;
    let bytes = to_bytes(response.unwrap().into_body(), 4 * 1024 * 1024).await;
    let served: Value = serde_json::from_slice(&bytes.unwrap()).unwrap();
    assert!(served["paths"]["/api/admin/v1/search-console/follow-ups/"]["get"].is_object());
}
