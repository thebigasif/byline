//! Photo and GIF search through the Admin API, with Unsplash stood in for
//! by `wiremock` (RULES.md BL-TST-3). Every key here is made up.

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
use khalid_media::{Access, Library, StockApis, StockSearch, Storage};
use khalid_outbound::{Limits, Outbound, Reach, Resolver};
use serde_json::{Value, json};
use tower::ServiceExt;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

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

/// Photo search asking `server`, with GIFs as `gifs` allows.
#[cfg(test)]
fn search(server: &MockServer, unsplash: Access, gifs: Access) -> StockSearch {
    let resolver = Resolver::for_tests(Vec::new(), Reach::PublicAndLoopback);
    let outbound = Outbound::with_resolver(Limits::default(), resolver).unwrap();
    let base = format!("{}/", server.uri());
    let apis = StockApis {
        unsplash: base.clone(),
        giphy: base,
    };
    StockSearch::new(outbound, unsplash)
        .with_gifs(gifs)
        .at(apis)
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
    let server = MockServer::start().await;
    let on = search(&server, keyed(), Access::Off);
    let searching = app(site.path(), Some(on)).await;
    let (status, body) = get(&searching, "/api/admin/v1/media/search/", UPLOADER).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body,
        json!({
            "unsplash": true,
            "gifs": false,
            "keys": { "unsplash": true, "gifs": false },
            "pictures": [],
        })
    );
    let bare = app(site.path(), None).await;
    let (_, body) = get(&bare, "/api/admin/v1/media/search/", UPLOADER).await;
    assert_eq!(body["unsplash"], false);
    assert_eq!(body["keys"]["unsplash"], false);
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn says_which_search_is_on_without_its_key() {
    let site = tempfile::tempdir().unwrap();
    let server = MockServer::start().await;
    let on = search(&server, Access::NoKey, keyed());
    let searching = app(site.path(), Some(on)).await;
    let (_, body) = get(&searching, "/api/admin/v1/media/search/", UPLOADER).await;
    assert_eq!(
        (&body["unsplash"], &body["gifs"]),
        (&json!(true), &json!(true))
    );
    assert_eq!(body["keys"], json!({ "unsplash": false, "gifs": true }));
}

#[tokio::test]
async fn finds_photos_for_a_post() {
    let site = tempfile::tempdir().unwrap();
    let server = MockServer::start().await;
    let answer = json!({ "results": [{
        "id": "Ab-12_c", "width": 1080, "height": 720,
        "description": "A lake", "alt_description": null,
        "urls": { "regular": "https://images.unsplash.com/1", "small": "https://images.unsplash.com/1s" },
        "links": { "html": "https://unsplash.com/photos/Ab-12_c" },
        "user": { "name": "Ana Silva", "links": { "html": "https://unsplash.com/@ana" } }
    }]});
    Mock::given(method("GET"))
        .and(path("/search/photos"))
        .and(query_param("query", "mountain lake"))
        .respond_with(ResponseTemplate::new(200).set_body_json(answer))
        .expect(1)
        .mount(&server)
        .await;
    let app = app(site.path(), Some(search(&server, keyed(), Access::Off))).await;
    let uri = "/api/admin/v1/media/search/?source=unsplash&words=mountain%20lake";
    let (status, body) = get(&app, uri, UPLOADER).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let photo = &body["pictures"][0];
    assert_eq!(photo["source"], "unsplash");
    assert_eq!(photo["address"], "https://images.unsplash.com/1");
    assert_eq!(photo["description"], "A lake");
    assert_eq!(photo["credit"], "Ana Silva");
    assert_eq!(
        (photo["width"].as_u64(), photo["height"].as_u64()),
        (Some(1080), Some(720))
    );
}

#[tokio::test]
async fn says_how_to_turn_a_search_on_or_give_it_its_key() {
    let site = tempfile::tempdir().unwrap();
    let server = MockServer::start().await;
    let keyless = search(&server, Access::on_with(None), Access::Off);
    let app = app(site.path(), Some(keyless)).await;
    let gifs = "/api/admin/v1/media/search/?source=gifs&words=dance";
    let (status, body) = get(&app, gifs, UPLOADER).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
    let detail = body["detail"].as_str().unwrap();
    assert!(
        detail.starts_with("GIF search on GIPHY is turned off"),
        "{body}"
    );
    assert!(
        body["fix"].as_str().unwrap().contains("gifs = true"),
        "{body}"
    );
    let photos = "/api/admin/v1/media/search/?source=unsplash&words=lake";
    let (status, body) = get(&app, photos, UPLOADER).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
    let fix = body["fix"].as_str().unwrap();
    assert!(fix.contains("KHALID_UNSPLASH_ACCESS_KEY"), "{body}");
    assert!(server.received_requests().await.unwrap().is_empty());
}

/// Posts `body` to the used route with `key`.
#[cfg(test)]
async fn used(app: &Router, key: &str, body: Value) -> (StatusCode, Value) {
    let request = Request::post("/api/admin/v1/media/search/used/")
        .header("authorization", format!("Khalid {key}"))
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
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
async fn tells_unsplash_when_a_photo_goes_into_a_post() {
    let site = tempfile::tempdir().unwrap();
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/photos/Ab-12_c/download"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    let app = app(site.path(), Some(search(&server, keyed(), Access::Off))).await;
    let (status, body) = used(&app, UPLOADER, json!({ "id": "Ab-12_c" })).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body, json!({ "id": "Ab-12_c" }));
    let (status, body) = used(&app, UPLOADER, json!({ "id": "../x" })).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    let (status, _) = used(&app, UPLOADER, json!({})).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (status, _) = used(&app, WRITER, json!({ "id": "Ab-12_c" })).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn a_photo_used_on_a_blog_without_search_says_so() {
    let site = tempfile::tempdir().unwrap();
    let app = app(site.path(), None).await;
    let (status, body) = used(&app, UPLOADER, json!({ "id": "Ab-12_c" })).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
    let detail = body["detail"].as_str().unwrap();
    assert!(
        detail.starts_with("Photo search on Unsplash is turned off"),
        "{body}"
    );
}

#[tokio::test]
async fn refuses_words_without_a_source_and_a_key_without_media_write() {
    let site = tempfile::tempdir().unwrap();
    let app = app(site.path(), None).await;
    let (status, _) = get(&app, "/api/admin/v1/media/search/?words=lake", UPLOADER).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = get(&app, "/api/admin/v1/media/search/?source=flickr", UPLOADER).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = get(&app, "/api/admin/v1/media/search/", WRITER).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
