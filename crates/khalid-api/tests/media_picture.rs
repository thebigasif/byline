//! Turned and cropped copies of library pictures through the Admin API
//! (plan 4.6, decisions E9 and E10).

mod common;

use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use image::{ImageFormat, Rgb, RgbImage};
use khalid_api::api;
use khalid_api::comment_moderation::NoComments;
use khalid_api::keys::{KeyHashing, KeyToken, MemoryKeyStore, Scope, StoredKey};
use khalid_api::media_api::MediaLibrary;
use khalid_api::post_store::MemoryPostStore;
use khalid_api::publish_door::{PublishDoor, Silent};
use khalid_core::{Locale, SystemClock};
use khalid_media::{Library, Storage};
use serde_json::{Value, json};
use tower::ServiceExt;

const UPLOADER: &str =
    "ki_2222333344445555_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";

#[cfg(test)]
async fn app(site: &Path) -> Router {
    let token = UPLOADER.parse::<KeyToken>().unwrap();
    let hashing = KeyHashing::fast_for_tests();
    let key = StoredKey::for_integration(&token, vec![Scope::MediaWrite], &hashing);
    let keys = MemoryKeyStore::new(vec![key.unwrap()]);
    let door = PublishDoor::new(Arc::new(MemoryPostStore::default()), Arc::new(Silent));
    let english = Locale::try_new("en").unwrap();
    let library = Library::open(site, Storage::local(site), &english, Arc::new(SystemClock));
    let library = Arc::new(library.await.unwrap());
    let media = MediaLibrary::new(library, "https://www.example.com/blog");
    let (content, door, keys) = (Arc::new(common::store()), Arc::new(door), Arc::new(keys));
    api::router_with_media(content, door, keys, Arc::new(NoComments), media).0
}

#[cfg(test)]
async fn send(app: &Router, uri: &str, key: &str, body: Vec<u8>) -> (StatusCode, Value) {
    let request = Request::post(uri)
        .header("authorization", format!("Khalid {key}"))
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// Uploads a 400 by 200 PNG and answers its identity.
#[cfg(test)]
async fn picture(app: &Router) -> String {
    let image = RgbImage::from_pixel(400, 200, Rgb([20, 90, 160]));
    let mut png = Cursor::new(Vec::new());
    image.write_to(&mut png, ImageFormat::Png).unwrap();
    let uri = "/api/admin/v1/media/?name=Team%20photo.png";
    let (status, body) = send(app, uri, UPLOADER, png.into_inner()).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    body["media"][0]["id"].as_str().unwrap().to_owned()
}

#[cfg(test)]
async fn copy(app: &Router, id: &str, key: &str, edit: &Value) -> (StatusCode, Value) {
    let uri = format!("/api/admin/v1/media/{id}/copies/");
    send(app, &uri, key, edit.to_string().into_bytes()).await
}

#[tokio::test]
async fn a_turned_and_cropped_copy_is_a_new_item() {
    let site = tempfile::tempdir().unwrap();
    let app = app(site.path()).await;
    let id = picture(&app).await;
    let edit = json!({ "turn": 90, "crop": { "left": 0, "top": 0, "width": 1000, "height": 500 } });
    let (status, body) = copy(&app, &id, UPLOADER, &edit).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let made = &body["media"][0];
    assert_eq!(
        (made["width"].as_u64(), made["height"].as_u64()),
        (Some(200), Some(200))
    );
    assert_eq!(made["name"], "team-photo.png");
    let (status, again) = copy(&app, &id, UPLOADER, &edit).await;
    assert_eq!(status, StatusCode::OK, "the same edit finds the first copy");
    assert_eq!(&again["media"][0], made);
}
