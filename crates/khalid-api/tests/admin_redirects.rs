//! The redirect routes of the Admin API (KH-SEO-8): stored redirects, the
//! debug view, CSV import and export, and the 404 log.

mod common;

use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use chrono::{TimeZone, Utc};
use khalid_api::api;
use khalid_api::keys::{KeyHashing, KeyToken, MemoryKeyStore, Scope, StoredKey};
use khalid_api::not_found_log::MemoryNotFoundLog;
use khalid_api::post_store::MemoryPostStore;
use khalid_api::publish_door::{PublishDoor, Silent};
use khalid_api::redirect_desk::{RedirectDesk, StaticPages};
use khalid_api::redirect_store::{MemoryRedirectStore, Origin, RedirectStore};
use khalid_core::FixedClock;
use khalid_seo::read_redirects_csv;
use serde_json::Value;
use tower::ServiceExt;

/// Keys made up for these tests. They open nothing real.
const SETTINGS: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const POSTS_ONLY: &str =
    "ki_5555666677778888_0000111122223333444455556666777788889999aaaabbbbccccddddeeeeffff";

/// A site with the redirects in `csv` stored by hand at nine o'clock.
#[cfg(test)]
async fn desk(csv: &str) -> RedirectDesk {
    let store = MemoryRedirectStore::default();
    let at = Utc.with_ymd_and_hms(2026, 11, 3, 9, 0, 0).unwrap();
    let entries = read_redirects_csv(&format!("from,to,status,match\n{csv}")).unwrap();
    store.add(entries, Origin::Manual, at).await.unwrap();
    let pages = [
        "/blog/",
        "/blog/why-internal-links-matter/",
        "/blog/robots-txt/",
    ];
    RedirectDesk::new(
        Arc::new(store),
        Arc::new(MemoryNotFoundLog::default()),
        Arc::new(StaticPages::new(&pages).unwrap()),
        Arc::new(FixedClock::new(at)),
    )
}

/// Sends a request with `key` to the API over `desk`, answering the status
/// and the body as JSON, or as a JSON string when it is not JSON.
#[cfg(test)]
async fn send(desk: RedirectDesk, key: &str, request: Request<Body>) -> (StatusCode, Value) {
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
    let (router, _) = api::router_with_redirects(
        Arc::new(common::store()),
        Arc::new(door),
        Arc::new(keys),
        desk,
    );
    let (mut parts, body) = request.into_parts();
    let authorization = format!("Khalid {key}").parse().unwrap();
    parts.headers.insert("authorization", authorization);
    let response = router
        .oneshot(Request::from_parts(parts, body))
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    (
        status,
        serde_json::from_str(&text).unwrap_or(Value::String(text)),
    )
}

#[cfg(test)]
fn get(uri: &str) -> Request<Body> {
    Request::get(uri).body(Body::empty()).unwrap()
}

#[tokio::test]
async fn the_list_shows_every_stored_redirect_with_its_columns() {
    let desk = desk(
        "/blog/old-links/,/blog/why-internal-links-matter/,301\n\
         /blog/retired-tool/,,410\n\
         /blog/\\d{4}/(.+),/blog/$1,302,regex\n",
    )
    .await;
    let (status, json) = send(desk, SETTINGS, get("/api/admin/v1/redirects/")).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    let first = &json["redirects"][0];
    assert_eq!(first["from"], "/blog/old-links/");
    assert_eq!(first["to"], "/blog/why-internal-links-matter/");
    assert_eq!(first["status"], 301);
    assert_eq!(first["match"], "exact");
    assert_eq!(first["origin"], "manual");
    assert_eq!(first["created_at"], "2026-11-03T09:00:00Z");
    assert!(first["id"].is_i64(), "{json}");
    assert_eq!(json["redirects"][1]["to"], Value::Null);
    assert_eq!(json["redirects"][1]["status"], 410);
    assert_eq!(json["redirects"][2]["match"], "regex");
    assert_eq!(json["redirects"][2]["to"], "/blog/$1");
}

#[tokio::test]
async fn redirects_need_the_settings_scope() {
    let desk = desk("/blog/sale/,/blog/offers/,302\n").await;
    let (status, json) = send(desk, POSTS_ONLY, get("/api/admin/v1/redirects/")).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{json}");
    assert_eq!(json["rule"], "settings:write");
}

#[tokio::test]
async fn a_site_without_a_redirect_store_lists_none() {
    let (status, json) = send(
        RedirectDesk::none(),
        SETTINGS,
        get("/api/admin/v1/redirects/"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["redirects"], Value::Array(Vec::new()));
}
