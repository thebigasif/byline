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
    let request = Request::post(uri)
        .header("authorization", format!("Khalid {key}"))
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
