//! The Admin API makes a preview link for a post: a signed address that
//! checks out until it expires, and none for a post that does not exist
//! (CTO answers 2026-10-27 section 4).

mod common;

use std::sync::Arc;
use std::time::Duration;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use chrono::{DateTime, TimeZone, Utc};
use khalid_api::api;
use khalid_api::keys::{KeyHashing, KeyToken, MemoryKeyStore, Scope, StoredKey};
use khalid_api::post_store::MemoryPostStore;
use khalid_api::preview_link::PreviewSigner;
use khalid_api::publish_door::{PublishDoor, Silent};
use khalid_api::resources::AdminPost;
use khalid_core::{AbsoluteUrl, FixedClock, Secret, Slug};
use serde_json::Value;
use tower::ServiceExt;

/// A key made up for these tests. It opens nothing real.
const WRITER: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";

#[cfg(test)]
fn at(day: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, day, 9, 0, 0).unwrap()
}

/// A signer for the demo blog with a made-up key on `day` October,
/// making links that work for 7 days.
#[cfg(test)]
fn signer(day: u32) -> PreviewSigner {
    let blog = AbsoluteUrl::parse("https://www.example.com/blog/").unwrap();
    let key = Secret::new([7_u8; 32]);
    let week = Duration::from_hours(7 * 24);
    PreviewSigner::new(&key, &blog, week, FixedClock::new(at(day)))
}

/// Asks the Admin API for a preview link to post `id`.
#[cfg(test)]
async fn ask(id: &str) -> (StatusCode, Value) {
    let draft = AdminPost::new(
        "en.choosing-anchor-text",
        Slug::try_new("choosing-anchor-text").unwrap(),
        "Choosing anchor text",
        "Words.",
        at(20),
    );
    let posts = Arc::new(MemoryPostStore::new(vec![draft]));
    let door = PublishDoor::new(posts, Arc::new(Silent)).with_previews(signer(27));
    let token: KeyToken = WRITER.parse().unwrap();
    let hashing = KeyHashing::fast_for_tests();
    let keys = MemoryKeyStore::new(vec![
        StoredKey::for_integration(&token, vec![Scope::PostsWrite], &hashing).unwrap(),
    ]);
    let content = Arc::new(common::store());
    let (router, _) = api::router_with_door(content, Arc::new(door), Arc::new(keys));
    let request = Request::post(format!("/api/admin/v1/posts/{id}/preview-link/"))
        .header("authorization", format!("Khalid {WRITER}"))
        .body(Body::empty())
        .unwrap();
    let response = router.oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn the_admin_api_makes_a_link_for_a_post() {
    let (status, json) = ask("en.choosing-anchor-text").await;
    assert_eq!(status, StatusCode::CREATED, "{json}");
    let link = &json["preview_links"][0];
    assert_eq!(link["expires_at"], "2026-11-03T09:00:00Z", "{json}");
    let query = link["url"].as_str().unwrap().split_once('?').unwrap().1;
    let (until, sig) = query.split_once("&sig=").unwrap();
    let until = until.strip_prefix("until=").unwrap().parse().unwrap();
    let checked = signer(28).check("en.choosing-anchor-text", until, sig);
    assert_eq!(checked, Ok(()));
}

#[tokio::test]
async fn no_link_is_made_for_a_post_that_does_not_exist() {
    let (status, json) = ask("en.no-such-post").await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{json}");
}
