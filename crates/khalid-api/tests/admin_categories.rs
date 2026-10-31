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
const ON_PAGE: &str = "/api/admin/v1/categories/on-page-seo/";
const LINKS: &str = "/api/admin/v1/categories/link-building/";
const LAST_CHANGE: &str = "2026-10-20T09:00:00Z";

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

#[tokio::test]
async fn one_category_is_read_by_its_slug() {
    let router = with_two();
    let (status, json) = send(&router, "GET", ON_PAGE, WRITER, None).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(
        json["categories"][0]["names"]["en"], "On-page SEO",
        "{json}"
    );
    let missing = "/api/admin/v1/categories/nothing/";
    let (status, json) = send(&router, "GET", missing, WRITER, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{json}");
    assert!(
        json["fix"].as_str().unwrap().contains("on-page-seo"),
        "{json}"
    );
}

#[tokio::test]
async fn a_new_category_is_made_once_with_its_names() {
    let router = with_two();
    let new =
        json!({"categories": [{"slug": "content-strategy", "names": {"en": "Content strategy"}}]});
    let (status, json) = send(&router, "POST", CATEGORIES, EDITOR, Some(new.clone())).await;
    assert_eq!(status, StatusCode::CREATED, "{json}");
    assert_eq!(json["categories"][0]["id"], "content-strategy", "{json}");
    assert_eq!(json["categories"][0]["posts"], 0, "{json}");
    let (status, json) = send(&router, "POST", CATEGORIES, EDITOR, Some(new)).await;
    assert_eq!(status, StatusCode::CONFLICT, "{json}");
}

#[tokio::test]
async fn a_category_that_cannot_be_kept_is_refused() {
    let router = with_two();
    let cases = [
        json!({"categories": [{"slug": "news", "names": {}}]}),
        json!({"categories": [{"slug": "news", "names": {"english": "News"}}]}),
        json!({"categories": [{"slug": "news", "names": {"en": ""}}]}),
        json!({"categories": [{"slug": "News!", "names": {"en": "News"}}]}),
    ];
    for body in cases {
        let (status, json) = send(&router, "POST", CATEGORIES, EDITOR, Some(body.clone())).await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{body} gave {json}"
        );
    }
    let new = json!({"categories": [{"slug": "news", "names": {"en": "News"}}]});
    let (status, json) = send(&router, "POST", CATEGORIES, WRITER, Some(new)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{json}");
    assert_eq!(json["rule"], "posts:publish", "{json}");
}

#[tokio::test]
async fn an_edit_replaces_the_names_on_the_latest_copy() {
    let router = with_two();
    let edit = json!({"categories": [{"names": {"en": "On-page"}, "updated_at": LAST_CHANGE}]});
    let (status, json) = send(&router, "PUT", ON_PAGE, EDITOR, Some(edit.clone())).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(
        json["categories"][0]["names"],
        json!({"en": "On-page"}),
        "{json}"
    );
    let (status, json) = send(&router, "PUT", ON_PAGE, EDITOR, Some(edit)).await;
    assert_eq!(status, StatusCode::CONFLICT, "{json}");
    assert_eq!(json["rule"], "updated_at", "{json}");
    let empty = json!({"categories": [{"names": {}, "updated_at": LAST_CHANGE}]});
    let (status, json) = send(&router, "PUT", LINKS, EDITOR, Some(empty)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{json}");
    assert_eq!(json["rule"], "names", "{json}");
}

#[tokio::test]
async fn a_category_with_posts_is_kept_and_an_empty_one_is_deleted() {
    let router = with_two();
    let on_page = format!("{ON_PAGE}?updated_at={LAST_CHANGE}");
    let (status, json) = send(&router, "DELETE", &on_page, EDITOR, None).await;
    assert_eq!(status, StatusCode::CONFLICT, "{json}");
    assert_eq!(json["rule"], "posts", "{json}");
    let (status, _) = send(&router, "DELETE", LINKS, EDITOR, None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let links = format!("{LINKS}?updated_at={LAST_CHANGE}");
    let (status, _) = send(&router, "DELETE", &links, WRITER, None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = send(&router, "DELETE", &links, EDITOR, None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = send(&router, "GET", LINKS, EDITOR, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
