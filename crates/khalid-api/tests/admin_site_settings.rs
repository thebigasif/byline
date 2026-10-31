//! The site's core settings through the Admin API, with the 409 a stale
//! copy gets and the 422 a value the site could not start with gets
//! (KH-API-2).

use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use chrono::{TimeZone, Utc};
use khalid_api::api::{self, AdminDesks};
use khalid_api::content_store::MemoryContentStore;
use khalid_api::keys::{KeyHashing, KeyToken, MemoryKeyStore, Scope, StoredKey};
use khalid_api::post_store::MemoryPostStore;
use khalid_api::publish_door::{PublishDoor, Silent};
use khalid_api::resources::AdminSettings;
use khalid_api::settings_store::{MemorySettings, SettingsStore};
use khalid_core::FixedClock;
use serde_json::{Value, json};
use tower::ServiceExt;

/// Keys made up for these tests. They open nothing real.
const OWNER: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const EDITOR: &str =
    "ki_2222333344445555_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";

const SETTINGS: &str = "/api/admin/v1/settings/";
const LAST_CHANGE: &str = "2026-10-20T09:00:00Z";

/// A site keeping `settings`, or none at all.
#[cfg(test)]
fn router(settings: Option<Arc<dyn SettingsStore>>) -> Router {
    let hashing = KeyHashing::fast_for_tests();
    let token = |text: &str| text.parse::<KeyToken>().unwrap();
    let key = |text, scopes| StoredKey::for_integration(&token(text), scopes, &hashing).unwrap();
    let keys = MemoryKeyStore::new(vec![
        key(OWNER, vec![Scope::SettingsWrite]),
        key(EDITOR, vec![Scope::PostsWrite, Scope::PostsPublish]),
    ]);
    let desks = AdminDesks {
        settings,
        ..AdminDesks::none()
    };
    let door = PublishDoor::new(Arc::new(MemoryPostStore::default()), Arc::new(Silent));
    let content = Arc::new(MemoryContentStore::new(Vec::new()));
    let (router, _) = api::router_with_desks(content, Arc::new(door), Arc::new(keys), desks);
    router
}

/// The demo's settings, last changed on 20 October.
#[cfg(test)]
fn demo() -> Router {
    let settings = AdminSettings {
        name: "Field Notes on SEO".to_owned(),
        address: "https://www.example.com".to_owned(),
        mount: "/blog/".to_owned(),
        default_language: "en".to_owned(),
        languages: vec!["en".to_owned(), "de".to_owned()],
        updated_at: Utc.with_ymd_and_hms(2026, 10, 20, 9, 0, 0).unwrap(),
        restart_needed: false,
    };
    let now = Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap();
    let store = MemorySettings::new(settings).with_clock(FixedClock::new(now));
    router(Some(Arc::new(store)))
}

#[cfg(test)]
async fn send(
    router: &Router,
    method: &str,
    key: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(method)
        .uri(SETTINGS)
        .header("authorization", format!("Khalid {key}"))
        .header("content-type", "application/json");
    let body = body.map_or_else(Body::empty, |body| Body::from(body.to_string()));
    let request = request.body(body).unwrap();
    let response = router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn the_settings_are_read_with_settings_write() {
    let (status, json) = send(&demo(), "GET", OWNER, None).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["settings"]["mount"], "/blog/", "{json}");
    assert_eq!(json["settings"]["languages"], json!(["en", "de"]), "{json}");
    assert_eq!(json["settings"]["updated_at"], LAST_CHANGE, "{json}");
}

#[tokio::test]
async fn settings_need_settings_write_and_a_server_that_keeps_them() {
    let (status, json) = send(&demo(), "GET", EDITOR, None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{json}");
    assert_eq!(json["rule"], "settings:write", "{json}");
    let (status, json) = send(&router(None), "GET", OWNER, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{json}");
}
