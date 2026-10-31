//! Photo and GIF search through the Admin API. Every key here is made up.

mod common;

use std::path::Path;
use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use khalid_api::api;
use khalid_api::comment_moderation::NoComments;
use khalid_api::keys::{KeyHashing, KeyToken, MemoryKeyStore, Scope, StoredKey};
use khalid_api::media_api::MediaLibrary;
use khalid_api::post_store::MemoryPostStore;
use khalid_api::publish_door::{PublishDoor, Silent};
use khalid_core::{Locale, Secret, SystemClock};
use khalid_media::{Access, Library, StockSearch, Storage};
use khalid_outbound::{Limits, Outbound};
use serde_json::{Value, json};
use tower::ServiceExt;

const UPLOADER: &str =
    "ki_2222333344445555_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const WRITER: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";

/// The API with a media library in `site` and `search`, if any.
#[cfg(test)]
async fn app(site: &Path, search: Option<StockSearch>) -> Router {
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
    let english = Locale::try_new("en").unwrap();
    let library = Library::open(site, Storage::local(site), &english, Arc::new(SystemClock));
    let mut media = MediaLibrary::new(Arc::new(library.await.unwrap()), "https://www.example.com/");
    if let Some(search) = search {
        media = media.with_search(search);
    }
    let (content, door, keys) = (Arc::new(common::store()), Arc::new(door), Arc::new(keys));
    api::router_with_media(content, door, keys, Arc::new(NoComments), media).0
}

#[cfg(test)]
fn keyed() -> Access {
    Access::on_with(Some(Secret::new("made-for-this-test".to_owned())))
}

#[cfg(test)]
async fn get(app: &Router, uri: &str, key: &str) -> (StatusCode, Value) {
    let request = Request::get(uri)
        .header("authorization", format!("Khalid {key}"))
        .body(Body::empty())
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
async fn says_which_searches_are_on_without_asking_anyone() {
    let site = tempfile::tempdir().unwrap();
    let outbound = Outbound::new(Limits::default()).unwrap();
    let on = StockSearch::new(outbound, keyed());
    let searching = app(site.path(), Some(on)).await;
    let (status, body) = get(&searching, "/api/admin/v1/media/search/", UPLOADER).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body, json!({ "unsplash": true, "gifs": false }));
    let bare = app(site.path(), None).await;
    let (_, body) = get(&bare, "/api/admin/v1/media/search/", UPLOADER).await;
    assert_eq!(body, json!({ "unsplash": false, "gifs": false }));
}

#[tokio::test]
async fn refuses_a_key_without_media_write() {
    let site = tempfile::tempdir().unwrap();
    let app = app(site.path(), None).await;
    let (status, _) = get(&app, "/api/admin/v1/media/search/", WRITER).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
