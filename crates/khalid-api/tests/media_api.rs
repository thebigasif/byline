//! Uploads to the media library through the Admin API.

mod common;

use std::path::Path;
use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use chrono::{TimeZone, Utc};
use khalid_api::api;
use khalid_api::comment_moderation::NoComments;
use khalid_api::keys::{KeyHashing, KeyToken, MemoryKeyStore, Scope, StoredKey};
use khalid_api::media_api::MediaLibrary;
use khalid_api::post_store::MemoryPostStore;
use khalid_api::publish_door::{PublishDoor, Silent};
use khalid_core::{FixedClock, Locale};
use khalid_media::{Library, Storage};
use serde_json::Value;
use tower::ServiceExt;

/// Keys made up for these tests. They open nothing real.
const UPLOADER: &str =
    "ki_2222333344445555_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const WRITER: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";

const PDF: &[u8] = b"%PDF-1.7\nA checklist for choosing a CRM\n%%EOF\n";
const UPLOAD: &str = "/api/admin/v1/media/?name=CRM%20checklist.pdf&folder=guides";

/// The API with the media library of `site`, or with none.
#[cfg(test)]
async fn app(site: Option<&Path>) -> Router {
    let hashing = KeyHashing::fast_for_tests();
    let key = |text: &str, scope| {
        let token = text.parse::<KeyToken>().unwrap();
        StoredKey::for_integration(&token, vec![scope], &hashing).unwrap()
    };
    let keys = MemoryKeyStore::new(vec![
        key(UPLOADER, Scope::MediaWrite),
        key(WRITER, Scope::PostsWrite),
    ]);
    let door = PublishDoor::new(Arc::new(MemoryPostStore::default()), Arc::new(Silent));
    let (content, door, keys) = (Arc::new(common::store()), Arc::new(door), Arc::new(keys));
    let Some(site) = site else {
        return api::router_with_door(content, door, keys).0;
    };
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 10, 27, 8, 0, 0).unwrap());
    let english = Locale::try_new("en").unwrap();
    let library = Library::open(site, Storage::local(site), &english, Arc::new(clock));
    let library = Arc::new(library.await.unwrap());
    let media = MediaLibrary::new(library, "https://www.example.com/blog");
    api::router_with_media(content, door, keys, Arc::new(NoComments), media).0
}

/// Posts `body` to `uri` with `key`, and reads the answer.
#[cfg(test)]
async fn post(app: &Router, uri: &str, key: &str, body: &[u8]) -> (StatusCode, Value) {
    send(app, "POST", uri, key, body).await
}

/// Sends `body` to `uri` with `method` and `key`, and reads the answer.
#[cfg(test)]
async fn send(
    app: &Router,
    method: &str,
    uri: &str,
    key: &str,
    body: &[u8],
) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Khalid {key}"))
        .header("content-type", "application/json")
        .body(Body::from(body.to_vec()))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn an_upload_is_added_once() {
    let site = tempfile::tempdir().unwrap();
    let app = app(Some(site.path())).await;
    let (status, body) = post(&app, UPLOAD, UPLOADER, PDF).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let item = &body["media"][0];
    let id = item["id"].as_str().unwrap();
    assert_eq!(item["folder"], "guides");
    let url = format!("https://www.example.com/blog/media/{id}/crm-checklist.pdf");
    assert_eq!(item["url"], url.as_str());
    let (status, again) = post(&app, "/api/admin/v1/media/?name=copy", UPLOADER, PDF).await;
    assert_eq!(status, StatusCode::OK, "the same file is not added twice");
    assert_eq!(&again["media"][0], item);
}

#[tokio::test]
async fn an_item_is_read_by_its_identity_and_nothing_else() {
    let site = tempfile::tempdir().unwrap();
    let app = app(Some(site.path())).await;
    let (_, body) = post(&app, UPLOAD, UPLOADER, PDF).await;
    let item = &body["media"][0];
    let uri = format!("/api/admin/v1/media/{}/", item["id"].as_str().unwrap());
    let (status, read) = send(&app, "GET", &uri, UPLOADER, b"").await;
    assert_eq!((status, &read["media"][0]), (StatusCode::OK, item));
    let others = [
        "/api/admin/v1/media/0000000000000000/",
        "/api/admin/v1/media/..%2Fsite.toml/",
    ];
    for uri in others {
        let (status, _) = send(&app, "GET", uri, UPLOADER, b"").await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
    }
}

#[tokio::test]
async fn refuses_a_file_the_library_must_not_keep_and_a_bad_folder() {
    let site = tempfile::tempdir().unwrap();
    let app = app(Some(site.path())).await;
    let html = b"<!doctype html><script>steal()</script>";
    let (status, body) = post(&app, UPLOAD, UPLOADER, html).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["rule"], "media");
    let detail = body["detail"].as_str().unwrap();
    let why = "The file was not added: the file is HTML, SVG or XML.";
    assert!(detail.starts_with(why), "{detail}");
    let (status, body) = post(&app, "/api/admin/v1/media/?folder=../up", UPLOADER, PDF).await;
    assert_eq!(
        (status, &body["rule"]),
        (StatusCode::BAD_REQUEST, &"folder".into())
    );
}

#[tokio::test]
async fn needs_a_key_with_media_write_and_a_library() {
    let site = tempfile::tempdir().unwrap();
    let app_with_library = app(Some(site.path())).await;
    let (status, _) = post(&app_with_library, UPLOAD, WRITER, PDF).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, body) = post(&app(None).await, UPLOAD, UPLOADER, PDF).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let detail = body["detail"].as_str().unwrap();
    assert!(
        detail.starts_with("This site has no media library."),
        "{detail}"
    );
}

#[tokio::test]
async fn lists_the_library_with_the_shared_grammar() {
    let site = tempfile::tempdir().unwrap();
    let app = app(Some(site.path())).await;
    let notes = b"Keyword research checklist\n1. Seed terms\n2. Search intent\n";
    let zip = b"PK\x03\x04\x14\x00\x00\x00\x08\x00";
    post(&app, UPLOAD, UPLOADER, PDF).await;
    post(
        &app,
        "/api/admin/v1/media/?name=notes&folder=notes",
        UPLOADER,
        notes,
    )
    .await;
    post(
        &app,
        "/api/admin/v1/media/?name=pack&folder=notes",
        UPLOADER,
        zip,
    )
    .await;
    let list = |query: &str| {
        let uri = format!("/api/admin/v1/media/?{query}");
        let app = app.clone();
        async move { send(&app, "GET", &uri, UPLOADER, b"").await }
    };
    let names = |body: &Value| -> Vec<String> {
        let items = body["media"].as_array().unwrap();
        items
            .iter()
            .map(|item| item["name"].as_str().unwrap().to_owned())
            .collect()
    };
    let (status, all) = list("").await;
    assert_eq!(status, StatusCode::OK, "{all}");
    assert_eq!(all["meta"]["pagination"]["total"], 3);
    let (_, in_notes) = list("filter=folder:notes&order=bytes%20desc").await;
    assert_eq!(names(&in_notes), ["notes.txt", "pack.zip"]);
    let (_, first) = list("filter=alt:null&order=name&limit=1").await;
    assert_eq!(names(&first), ["crm-checklist.pdf"]);
    assert_eq!(first["meta"]["pagination"]["pages"], 3);
    let (status, body) = list("filter=colour:blue").await;
    assert_eq!(
        (status, &body["rule"]),
        (StatusCode::BAD_REQUEST, &"filter".into())
    );
}

/// The body of an edit of the copy changed at `updated_at`, with `fields`.
#[cfg(test)]
fn edit(updated_at: &Value, fields: &Value) -> Vec<u8> {
    let mut one = fields.clone();
    one["updated_at"] = updated_at.clone();
    serde_json::to_vec(&serde_json::json!({ "media": [one] })).unwrap()
}

/// Uploads the PDF and returns the item and its address in the API.
#[cfg(test)]
async fn uploaded(app: &Router) -> (Value, String) {
    let (_, body) = post(app, UPLOAD, UPLOADER, PDF).await;
    let item = body["media"][0].clone();
    let uri = format!("/api/admin/v1/media/{}/", item["id"].as_str().unwrap());
    (item, uri)
}

#[tokio::test]
async fn an_item_is_edited_and_an_edit_of_an_old_copy_is_refused() {
    let site = tempfile::tempdir().unwrap();
    let app = app(Some(site.path())).await;
    let (item, uri) = uploaded(&app).await;
    let fields = serde_json::json!({
        "alt": "A one-page checklist of questions for a CRM vendor",
        "focal_point": { "x": 250, "y": 100 },
        "folder": "guides/crm"
    });
    let change = edit(&item["updated_at"], &fields);
    let (status, saved) = send(&app, "PUT", &uri, UPLOADER, &change).await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    let saved = &saved["media"][0];
    assert_eq!(saved["alt"], fields["alt"]);
    assert_eq!(saved["focal_point"], fields["focal_point"]);
    assert_eq!(saved["folder"], fields["folder"]);
    assert_eq!(
        saved["url"], item["url"],
        "moving folders keeps the address"
    );
    let (status, body) = send(&app, "PUT", &uri, UPLOADER, &change).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["rule"], "updated_at");
}

#[tokio::test]
async fn refuses_an_edit_it_cannot_make() {
    let site = tempfile::tempdir().unwrap();
    let app = app(Some(site.path())).await;
    let (item, uri) = uploaded(&app).await;
    let at = &item["updated_at"];
    let wrong = [
        serde_json::json!({ "focal_point": { "x": 1001, "y": 0 } }),
        serde_json::json!({ "folder": "../up" }),
        serde_json::json!({ "alt": "word ".repeat(201) }),
        serde_json::json!({ "colour": "blue" }),
    ];
    for fields in wrong {
        let (status, _) = send(&app, "PUT", &uri, UPLOADER, &edit(at, &fields)).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{fields}");
    }
    let unknown = "/api/admin/v1/media/0000000000000000/";
    let (status, _) = send(&app, "PUT", unknown, UPLOADER, &edit(at, &Value::Null)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
