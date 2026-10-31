//! The site's authors through the Admin API: the people bylines, author
//! pages and the author schema name (KH-API-2).

use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use chrono::{TimeZone, Utc};
use khalid_api::api::{self, AdminDesks};
use khalid_api::content_store::MemoryContentStore;
use khalid_api::entry_store::{Entry, EntryStore, MemoryEntries};
use khalid_api::keys::{KeyHashing, KeyToken, MemoryKeyStore, Scope, StoredKey};
use khalid_api::post_store::MemoryPostStore;
use khalid_api::publish_door::{PublishDoor, Silent};
use khalid_api::resources::{AdminAuthor, NewAuthor};
use khalid_core::{FixedClock, Locale};
use serde_json::{Value, json};
use tower::ServiceExt;

/// Keys made up for these tests. They open nothing real.
const WRITER: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const EDITOR: &str =
    "ki_2222333344445555_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const READER: &str =
    "ki_3333444455556666_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";

const AUTHORS: &str = "/api/admin/v1/authors/";

/// A site keeping `authors`, or none at all.
#[cfg(test)]
fn router(authors: Option<Arc<dyn EntryStore<AdminAuthor>>>) -> Router {
    let hashing = KeyHashing::fast_for_tests();
    let token = |text: &str| text.parse::<KeyToken>().unwrap();
    let key = |text, scopes| StoredKey::for_integration(&token(text), scopes, &hashing).unwrap();
    let keys = MemoryKeyStore::new(vec![
        key(WRITER, vec![Scope::PostsWrite]),
        key(EDITOR, vec![Scope::PostsWrite, Scope::PostsPublish]),
        key(READER, vec![Scope::ContentRead]),
    ]);
    let desks = AdminDesks {
        authors,
        ..AdminDesks::none()
    };
    let door = PublishDoor::new(Arc::new(MemoryPostStore::default()), Arc::new(Silent));
    let content = Arc::new(MemoryContentStore::new(Vec::new()));
    let (router, _) = api::router_with_desks(content, Arc::new(door), Arc::new(keys), desks);
    router
}

/// A site with one author, Asif, last changed on 20 October.
#[cfg(test)]
fn with_asif() -> Router {
    let new: NewAuthor = serde_json::from_value(json!({
        "slug": "asif", "name": "Asif", "role": "Founder of SerpDad",
        "same_as": ["https://serpdad.com/"], "bio": "Asif writes about search.",
    }))
    .unwrap();
    let at = Utc.with_ymd_and_hms(2026, 10, 20, 9, 0, 0).unwrap();
    let asif = AdminAuthor::made(new, Locale::try_new("en").unwrap(), at);
    let now = Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap();
    let authors = MemoryEntries::new(vec![asif]).with_clock(FixedClock::new(now));
    router(Some(Arc::new(authors)))
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
async fn authors_are_listed_with_posts_write() {
    let router = with_asif();
    let (status, json) = send(&router, "GET", AUTHORS, WRITER, None).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["authors"][0]["id"], "en.asif", "{json}");
    assert_eq!(json["authors"][0]["same_as"][0], "https://serpdad.com/");
    assert_eq!(json["authors"][0]["bio"], "Asif writes about search.");
}

#[tokio::test]
async fn reading_authors_needs_a_key_with_posts_write() {
    let router = with_asif();
    let (status, json) = send(&router, "GET", AUTHORS, READER, None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{json}");
    assert_eq!(json["rule"], "posts:write", "{json}");
    let request = Request::get(AUTHORS).body(Body::empty()).unwrap();
    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn a_server_without_authors_answers_404_naming_why() {
    let (status, json) = send(&router(None), "GET", AUTHORS, EDITOR, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{json}");
    assert!(
        json["detail"].as_str().unwrap().contains("no authors"),
        "{json}"
    );
}

const ASIF: &str = "/api/admin/v1/authors/en.asif/";

#[tokio::test]
async fn one_author_is_read_by_their_identity() {
    let router = with_asif();
    let (status, json) = send(&router, "GET", ASIF, WRITER, None).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["authors"][0]["role"], "Founder of SerpDad", "{json}");
    let missing = "/api/admin/v1/authors/en.nobody/";
    let (status, json) = send(&router, "GET", missing, WRITER, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{json}");
    assert!(json["fix"].as_str().unwrap().contains("en.asif"), "{json}");
}

#[tokio::test]
async fn a_new_author_is_made_once_in_their_language() {
    let router = with_asif();
    let new = json!({"authors": [{"language": "de", "slug": "lena-vogt", "name": "Lena Vogt", "role": "Technical SEO editor"}]});
    let (status, json) = send(&router, "POST", AUTHORS, EDITOR, Some(new.clone())).await;
    assert_eq!(status, StatusCode::CREATED, "{json}");
    assert_eq!(json["authors"][0]["id"], "de.lena-vogt", "{json}");
    assert_eq!(json["authors"][0]["same_as"], json!([]), "{json}");
    let (status, json) = send(&router, "POST", AUTHORS, EDITOR, Some(new)).await;
    assert_eq!(status, StatusCode::CONFLICT, "{json}");
}

#[tokio::test]
async fn an_author_that_cannot_be_kept_or_is_sent_without_rights_is_refused() {
    let router = with_asif();
    let cases = [
        (
            json!({"authors": [{"slug": "lena", "name": "  "}]}),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            json!({"authors": [{"slug": "lena", "name": "Lena", "same_as": ["lena.example"]}]}),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            json!({"authors": [{"slug": "lena", "name": "Lena", "twitter": "@lena"}]}),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
    ];
    for (body, want) in cases {
        let (status, json) = send(&router, "POST", AUTHORS, EDITOR, Some(body.clone())).await;
        assert_eq!(status, want, "{body} gave {json}");
    }
    let new = json!({"authors": [{"slug": "lena", "name": "Lena"}]});
    let (status, json) = send(&router, "POST", AUTHORS, WRITER, Some(new)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{json}");
    assert_eq!(json["rule"], "posts:publish", "{json}");
    let (_, json) = send(&router, "GET", AUTHORS, WRITER, None).await;
    assert_eq!(
        json["authors"].as_array().map(Vec::len),
        Some(1),
        "nothing was saved: {json}"
    );
}

/// When Asif last changed.
const LAST_CHANGE: &str = "2026-10-20T09:00:00Z";

#[tokio::test]
async fn an_edit_on_the_latest_copy_changes_only_what_it_sends() {
    let router = with_asif();
    let edit = json!({"authors": [{"credentials": "Ten years in SEO", "role": "", "updated_at": LAST_CHANGE}]});
    let (status, json) = send(&router, "PUT", ASIF, EDITOR, Some(edit.clone())).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    let asif = &json["authors"][0];
    assert_eq!(asif["credentials"], "Ten years in SEO", "{json}");
    assert_eq!(
        asif["role"],
        json!(null),
        "an empty role removes it: {json}"
    );
    assert_eq!(asif["bio"], "Asif writes about search.", "{json}");
    assert_eq!(asif["updated_at"], "2026-10-27T09:00:00Z", "{json}");
    let (status, json) = send(&router, "PUT", ASIF, EDITOR, Some(edit)).await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "a replay of the old copy: {json}"
    );
    assert_eq!(json["rule"], "updated_at", "{json}");
}

#[tokio::test]
async fn an_edit_that_cannot_be_kept_is_refused() {
    let router = with_asif();
    let cases = [
        (
            json!({"authors": [{"name": "", "updated_at": LAST_CHANGE}]}),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            json!({"authors": [{"same_as": ["/about/"], "updated_at": LAST_CHANGE}]}),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            json!({"authors": [{"name": "Asif K"}]}),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
    ];
    for (body, want) in cases {
        let (status, json) = send(&router, "PUT", ASIF, EDITOR, Some(body.clone())).await;
        assert_eq!(status, want, "{body} gave {json}");
    }
    let edit = json!({"authors": [{"name": "Asif K", "updated_at": LAST_CHANGE}]});
    let (status, _) = send(&router, "PUT", ASIF, WRITER, Some(edit)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (_, json) = send(&router, "GET", ASIF, WRITER, None).await;
    assert_eq!(
        json["authors"][0]["name"], "Asif",
        "nothing was saved: {json}"
    );
}

#[tokio::test]
async fn an_author_is_deleted_only_on_their_latest_copy() {
    let router = with_asif();
    let (status, _) = send(&router, "DELETE", ASIF, EDITOR, None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let stale = format!("{ASIF}?updated_at=2026-10-19T09:00:00Z");
    let (status, _) = send(&router, "DELETE", &stale, EDITOR, None).await;
    assert_eq!(status, StatusCode::CONFLICT);
    let latest = format!("{ASIF}?updated_at={LAST_CHANGE}");
    let (status, _) = send(&router, "DELETE", &latest, WRITER, None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = send(&router, "DELETE", &latest, EDITOR, None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = send(&router, "GET", ASIF, EDITOR, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
