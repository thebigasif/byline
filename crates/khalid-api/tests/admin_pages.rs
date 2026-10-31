//! The site's pages through the Admin API: list, read, create, edit and
//! delete, with the 409 a stale copy gets (RULES.md BL-API-7, KH-API-2).

use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use chrono::{TimeZone, Utc};
use khalid_api::api::{self, AdminDesks};
use khalid_api::content_store::MemoryContentStore;
use khalid_api::entry_store::MemoryEntries;
use khalid_api::keys::{KeyHashing, KeyToken, MemoryKeyStore, Scope, StoredKey};
use khalid_api::post_store::MemoryPostStore;
use khalid_api::publish_door::{PublishDoor, Silent};
use khalid_api::resources::AdminPage;
use khalid_core::{FixedClock, Locale, Slug};
use serde_json::{Value, json};
use tower::ServiceExt;

/// Keys made up for these tests. They open nothing real.
const WRITER: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const EDITOR: &str =
    "ki_2222333344445555_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const READER: &str =
    "ki_3333444455556666_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";

#[cfg(test)]
fn router(desks: AdminDesks) -> Router {
    let hashing = KeyHashing::fast_for_tests();
    let token = |text: &str| text.parse::<KeyToken>().unwrap();
    let key = |text, scopes| StoredKey::for_integration(&token(text), scopes, &hashing).unwrap();
    let keys = MemoryKeyStore::new(vec![
        key(WRITER, vec![Scope::PostsWrite]),
        key(EDITOR, vec![Scope::PostsWrite, Scope::PostsPublish]),
        key(READER, vec![Scope::ContentRead]),
    ]);
    let posts = Arc::new(MemoryPostStore::default());
    let door = PublishDoor::new(posts, Arc::new(Silent));
    let content = MemoryContentStore::new(Vec::new());
    let (router, _) =
        api::router_with_desks(Arc::new(content), Arc::new(door), Arc::new(keys), desks);
    router
}

#[cfg(test)]
fn with_about() -> Router {
    let at = Utc.with_ymd_and_hms(2026, 10, 20, 9, 0, 0).unwrap();
    let key = (
        Locale::try_new("en").unwrap(),
        Slug::try_new("about").unwrap(),
    );
    let about = AdminPage::new(key, "About Field Notes", "Who writes here.", at);
    let now = Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap();
    let pages = MemoryEntries::new(vec![about]).with_clock(FixedClock::new(now));
    router(AdminDesks {
        pages: Some(Arc::new(pages)),
        ..AdminDesks::none()
    })
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
    let response = router
        .clone()
        .oneshot(request.body(body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

#[tokio::test]
async fn pages_are_listed_with_posts_write() {
    let router = with_about();
    let (status, json) = send(&router, "GET", "/api/admin/v1/pages/", WRITER, None).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["pages"][0]["id"], "en.about", "{json}");
    assert_eq!(json["pages"][0]["title"], "About Field Notes", "{json}");
    assert_eq!(json["pages"][0]["updated_at"], "2026-10-20T09:00:00Z");
}

#[tokio::test]
async fn a_new_page_is_live_in_the_default_language() {
    let router = with_about();
    let new =
        json!({"pages": [{"slug": "contact", "title": "Contact", "markdown": "Write to us."}]});
    let (status, json) = send(&router, "POST", "/api/admin/v1/pages/", EDITOR, Some(new)).await;
    assert_eq!(status, StatusCode::CREATED, "{json}");
    assert_eq!(json["pages"][0]["id"], "en.contact", "{json}");
    assert_eq!(json["pages"][0]["updated_at"], "2026-10-27T09:00:00Z");
    let (_, json) = send(&router, "GET", "/api/admin/v1/pages/", WRITER, None).await;
    assert_eq!(json["pages"][1]["id"], "en.contact", "{json}");
}

#[tokio::test]
async fn a_page_that_cannot_be_kept_is_refused() {
    let router = with_about();
    let cases = [
        (
            json!({"pages": [{"slug": "a", "title": " ", "markdown": "M."}]}),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            json!({"pages": [{"slug": "Not a slug", "title": "T", "markdown": "M."}]}),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (json!({"pages": []}), StatusCode::UNPROCESSABLE_ENTITY),
        (
            json!({"pages": [{"slug": "about", "title": "Again", "markdown": "M."}]}),
            StatusCode::CONFLICT,
        ),
    ];
    for (body, want) in cases {
        let pages = "/api/admin/v1/pages/";
        let (status, json) = send(&router, "POST", pages, EDITOR, Some(body.clone())).await;
        assert_eq!(status, want, "{body} gave {json}");
    }
    let (_, json) = send(&router, "GET", "/api/admin/v1/pages/", EDITOR, None).await;
    assert_eq!(json["pages"].as_array().map(Vec::len), Some(1), "{json}");
}

#[tokio::test]
async fn creating_a_page_needs_posts_publish() {
    let router = with_about();
    let new = json!({"pages": [{"slug": "contact", "title": "Contact", "markdown": "M."}]});
    let (status, json) = send(&router, "POST", "/api/admin/v1/pages/", WRITER, Some(new)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{json}");
    assert_eq!(json["rule"], "posts:publish", "{json}");
}

#[tokio::test]
async fn reading_pages_needs_posts_write_and_a_key() {
    let router = with_about();
    let (status, json) = send(&router, "GET", "/api/admin/v1/pages/", READER, None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{json}");
    assert_eq!(json["rule"], "posts:write", "{json}");
    let request = Request::get("/api/admin/v1/pages/")
        .body(Body::empty())
        .unwrap();
    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn a_server_without_pages_answers_404_naming_why() {
    let router = router(AdminDesks::none());
    let (status, json) = send(&router, "GET", "/api/admin/v1/pages/", EDITOR, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{json}");
    assert!(
        json["detail"].as_str().unwrap().contains("no pages"),
        "{json}"
    );
}

/// When About last changed, as an edit names it.
const LAST_CHANGE: &str = "2026-10-20T09:00:00Z";

#[tokio::test]
async fn one_page_is_read_by_its_identity() {
    let router = with_about();
    let about = "/api/admin/v1/pages/en.about/";
    let (status, json) = send(&router, "GET", about, WRITER, None).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["pages"][0]["markdown"], "Who writes here.", "{json}");
    let nothing = "/api/admin/v1/pages/en.nothing/";
    let (status, json) = send(&router, "GET", nothing, WRITER, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{json}");
    assert_eq!(json["instance"], nothing, "{json}");
}

#[tokio::test]
async fn an_edit_on_the_latest_copy_changes_only_what_it_sends() {
    let router = with_about();
    let about = "/api/admin/v1/pages/en.about/";
    let edit = json!({"pages": [{"title": "About us", "updated_at": LAST_CHANGE}]});
    let (status, json) = send(&router, "PUT", about, EDITOR, Some(edit)).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["pages"][0]["title"], "About us", "{json}");
    assert_eq!(json["pages"][0]["markdown"], "Who writes here.", "{json}");
    assert_eq!(json["pages"][0]["updated_at"], "2026-10-27T09:00:00Z");
}

#[tokio::test]
async fn a_stale_edit_is_a_409_and_changes_nothing() {
    let router = with_about();
    let about = "/api/admin/v1/pages/en.about/";
    let edit = json!({"pages": [{"title": "Lost", "updated_at": "2026-10-19T09:00:00Z"}]});
    let (status, json) = send(&router, "PUT", about, EDITOR, Some(edit)).await;
    assert_eq!(status, StatusCode::CONFLICT, "{json}");
    assert_eq!(json["rule"], "updated_at", "{json}");
    let (_, json) = send(&router, "GET", about, EDITOR, None).await;
    assert_eq!(json["pages"][0]["title"], "About Field Notes", "{json}");
}

#[tokio::test]
async fn an_edit_that_cannot_be_kept_is_refused() {
    let router = with_about();
    let about = "/api/admin/v1/pages/en.about/";
    let blank = json!({"pages": [{"title": " ", "updated_at": LAST_CHANGE}]});
    let (status, json) = send(&router, "PUT", about, EDITOR, Some(blank)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{json}");
    assert_eq!(json["rule"], "title", "{json}");
    let undated = json!({"pages": [{"title": "About us"}]});
    let (status, _) = send(&router, "PUT", about, EDITOR, Some(undated)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let edit = json!({"pages": [{"title": "About us", "updated_at": LAST_CHANGE}]});
    let (status, json) = send(&router, "PUT", about, WRITER, Some(edit.clone())).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{json}");
    let missing = "/api/admin/v1/pages/en.nothing/";
    let (status, _) = send(&router, "PUT", missing, EDITOR, Some(edit)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_page_is_deleted_only_on_its_latest_copy() {
    let router = with_about();
    let about = "/api/admin/v1/pages/en.about/";
    let stale = format!("{about}?updated_at=2026-10-19T09:00:00Z");
    let (status, json) = send(&router, "DELETE", &stale, EDITOR, None).await;
    assert_eq!(status, StatusCode::CONFLICT, "{json}");
    assert_eq!(json["rule"], "updated_at", "{json}");
    let latest = format!("{about}?updated_at={LAST_CHANGE}");
    let (status, json) = send(&router, "DELETE", &latest, WRITER, None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{json}");
    let (status, _) = send(&router, "DELETE", &latest, EDITOR, None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = send(&router, "GET", about, EDITOR, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = send(&router, "DELETE", &latest, EDITOR, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_delete_that_names_no_copy_is_a_400() {
    let router = with_about();
    let about = "/api/admin/v1/pages/en.about/";
    let (status, json) = send(&router, "DELETE", about, EDITOR, None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{json}");
    assert!(json["detail"].as_str().unwrap().contains("page"), "{json}");
    let garbled = format!("{about}?updated_at=yesterday");
    let (status, _) = send(&router, "DELETE", &garbled, EDITOR, None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = send(&router, "GET", about, EDITOR, None).await;
    assert_eq!(status, StatusCode::OK);
}
