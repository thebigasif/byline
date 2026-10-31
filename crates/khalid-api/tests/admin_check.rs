//! `GET /api/admin/v1/posts/{id}/check/`: a post's findings, score,
//! search snippet and schema graph before it goes live (G54, KH-SEO-1,
//! KH-SEO-4, KH-LNT-4).

mod common;

use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use khalid_api::api::{self, AdminDesks};
use khalid_api::keys::{KeyHashing, KeyToken, MemoryKeyStore, Scope, StoredKey};
use khalid_api::post_check::{MemoryPostChecks, PostCheck, PostChecks, Snippet};
use khalid_api::post_store::MemoryPostStore;
use khalid_api::publish_door::{PublishDoor, Silent};
use khalid_lint::Report;
use serde_json::{Value, json};
use tower::ServiceExt;

/// Keys made up for these tests. They open nothing real.
const WRITER: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const WEBHOOKS_ONLY: &str =
    "ki_5555666677778888_0000111122223333444455556666777788889999aaaabbbbccccddddeeeeffff";

/// The check of the draft `anchor-text`, with nothing found.
#[cfg(test)]
fn check() -> PostCheck {
    let snippet = Snippet {
        title: "Choosing anchor text | Field Notes on SEO".to_owned(),
        url: "https://www.example.com/blog/choosing-anchor-text/".to_owned(),
        description: "Pick words that say where the link goes.".to_owned(),
    };
    let graph = json!({"@context": "https://schema.org", "@graph": [{"@type": "BlogPosting"}]});
    PostCheck::new("anchor-text", Report::default(), snippet, graph)
}

/// Sends a check of post `id` with `key` to an API whose checks are
/// `checks`; answers the status and the body.
#[cfg(test)]
async fn send(checks: Option<Arc<dyn PostChecks>>, key: &str, id: &str) -> (StatusCode, Value) {
    let hashing = KeyHashing::fast_for_tests();
    let stored = |text: &str, scope| {
        let token: KeyToken = text.parse().unwrap();
        StoredKey::for_integration(&token, vec![scope], &hashing).unwrap()
    };
    let keys = MemoryKeyStore::new(vec![
        stored(WRITER, Scope::PostsWrite),
        stored(WEBHOOKS_ONLY, Scope::WebhooksManage),
    ]);
    let door = PublishDoor::new(Arc::new(MemoryPostStore::default()), Arc::new(Silent));
    let desks = AdminDesks {
        checks,
        ..AdminDesks::none()
    };
    let content = Arc::new(common::store());
    let (router, _) = api::router_with_desks(content, Arc::new(door), Arc::new(keys), desks);
    let request = Request::get(format!("/api/admin/v1/posts/{id}/check/"))
        .header("authorization", format!("Khalid {key}"))
        .body(Body::empty())
        .unwrap();
    let response = router.oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[cfg(test)]
fn checks() -> Arc<dyn PostChecks> {
    Arc::new(MemoryPostChecks::new(vec![check()]))
}

#[tokio::test]
async fn a_check_sends_the_findings_snippet_and_schema() {
    let (status, json) = send(Some(checks()), WRITER, "anchor-text").await;
    assert_eq!(status, StatusCode::OK, "{json}");
    let sent = &json["checks"][0];
    assert_eq!(sent["id"], "anchor-text");
    assert_eq!(sent["lint"], json!({"blocks": false, "findings": []}));
    let snippet = &sent["snippet"];
    assert_eq!(
        snippet["title"],
        "Choosing anchor text | Field Notes on SEO"
    );
    assert_eq!(
        snippet["url"],
        "https://www.example.com/blog/choosing-anchor-text/"
    );
    assert_eq!(sent["schema"]["@graph"][0]["@type"], "BlogPosting");
}

#[tokio::test]
async fn an_unknown_post_is_a_404() {
    let (status, json) = send(Some(checks()), WRITER, "nothing-here").await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{json}");
    assert_eq!(
        json["detail"],
        "No post has this identity. It may have been deleted, or the identity may be mistyped."
    );
}

#[tokio::test]
async fn a_server_without_checks_says_so() {
    let (status, json) = send(None, WRITER, "anchor-text").await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{json}");
    assert_eq!(json["fix"], "Send the request to the running khalid serve.");
}

#[tokio::test]
async fn a_key_that_cannot_write_posts_is_a_403() {
    let (status, json) = send(Some(checks()), WEBHOOKS_ONLY, "anchor-text").await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{json}");
}
