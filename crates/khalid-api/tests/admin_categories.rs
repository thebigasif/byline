//! The site's categories through the Admin API: one slug named in every
//! language, never deleted while posts are in it (KH-API-2).

use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use chrono::{TimeZone, Utc};
use khalid_api::api::{self, AdminDesks};
use khalid_api::content_store::MemoryContentStore;
use khalid_api::entry_store::{EntryStore, MemoryEntries};
use khalid_api::keys::{KeyHashing, KeyToken, MemoryKeyStore, Scope, StoredKey};
use khalid_api::post_store::MemoryPostStore;
use khalid_api::publish_door::{PublishDoor, Silent};
use khalid_api::resources::AdminCategory;
use khalid_core::{FixedClock, Slug};
use serde_json::{Value, json};
use tower::ServiceExt;

/// Keys made up for these tests. They open nothing real.
const WRITER: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const EDITOR: &str =
    "ki_2222333344445555_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const READER: &str =
    "ki_3333444455556666_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";

const CATEGORIES: &str = "/api/admin/v1/categories/";

/// A site keeping `categories`, or none at all.
#[cfg(test)]
fn router(categories: Option<Arc<dyn EntryStore<AdminCategory>>>) -> Router {
    let hashing = KeyHashing::fast_for_tests();
    let token = |text: &str| text.parse::<KeyToken>().unwrap();
    let key = |text, scopes| StoredKey::for_integration(&token(text), scopes, &hashing).unwrap();
    let keys = MemoryKeyStore::new(vec![
        key(WRITER, vec![Scope::PostsWrite]),
        key(EDITOR, vec![Scope::PostsWrite, Scope::PostsPublish]),
        key(READER, vec![Scope::ContentRead]),
    ]);
    let desks = AdminDesks {
        categories,
        ..AdminDesks::none()
    };
    let door = PublishDoor::new(Arc::new(MemoryPostStore::default()), Arc::new(Silent));
    let content = Arc::new(MemoryContentStore::new(Vec::new()));
    let (router, _) = api::router_with_desks(content, Arc::new(door), Arc::new(keys), desks);
    router
}

/// On-page SEO with three posts, and link building with none.
#[cfg(test)]
fn with_two() -> Router {
    let at = Utc.with_ymd_and_hms(2026, 10, 20, 9, 0, 0).unwrap();
    let category = |slug, names: Value, posts| {
        let names = serde_json::from_value(names).unwrap();
        AdminCategory::new(Slug::try_new(slug).unwrap(), names, posts, at)
    };
    let kept = vec![
        category(
            "on-page-seo",
            json!({"en": "On-page SEO", "de": "Onpage-SEO"}),
            3,
        ),
        category("link-building", json!({"en": "Link building"}), 0),
    ];
    let now = Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap();
    router(Some(Arc::new(
        MemoryEntries::new(kept).with_clock(FixedClock::new(now)),
    )))
}

#[cfg(test)]
async fn send(
    router: &Router,
    method: &str,
    uri: &str,
    key: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
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
async fn categories_are_listed_with_their_names_and_posts() {
    let (status, json) = send(&with_two(), "GET", CATEGORIES, WRITER, None).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["categories"][0]["names"]["de"], "Onpage-SEO", "{json}");
    assert_eq!(json["categories"][0]["posts"], 3, "{json}");
    assert_eq!(json["categories"][1]["id"], "link-building", "{json}");
}

#[tokio::test]
async fn reading_categories_needs_posts_write_and_a_server_with_them() {
    let (status, json) = send(&with_two(), "GET", CATEGORIES, READER, None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{json}");
    assert_eq!(json["rule"], "posts:write", "{json}");
    let (status, json) = send(&router(None), "GET", CATEGORIES, EDITOR, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{json}");
    assert!(
        json["detail"].as_str().unwrap().contains("no categories"),
        "{json}"
    );
}
