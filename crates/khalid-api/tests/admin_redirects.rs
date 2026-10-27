//! The redirect routes of the Admin API (KH-SEO-8): stored redirects, the
//! debug view, CSV import and export, and the 404 log.

mod common;

use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use chrono::{TimeZone, Utc};
use khalid_api::api;
use khalid_api::keys::{KeyHashing, KeyToken, MemoryKeyStore, Scope, StoredKey};
use khalid_api::not_found_log::{MemoryNotFoundLog, NotFoundLog};
use khalid_api::post_store::MemoryPostStore;
use khalid_api::publish_door::{PublishDoor, Silent};
use khalid_api::redirect_desk::{RedirectDesk, StaticPages};
use khalid_api::redirect_store::{MemoryRedirectStore, Origin, RedirectStore};
use khalid_core::FixedClock;
use khalid_seo::read_redirects_csv;
use serde_json::{Value, json};
use tower::ServiceExt;

/// Keys made up for these tests. They open nothing real.
const SETTINGS: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const POSTS_ONLY: &str =
    "ki_5555666677778888_0000111122223333444455556666777788889999aaaabbbbccccddddeeeeffff";

/// A site with the redirects in `csv` stored by hand at nine o'clock, and
/// three requests that found nothing.
#[cfg(test)]
async fn desk(csv: &str) -> RedirectDesk {
    let store = MemoryRedirectStore::default();
    let at = Utc.with_ymd_and_hms(2026, 11, 3, 9, 0, 0).unwrap();
    let entries = read_redirects_csv(&format!("from,to,status,match\n{csv}")).unwrap();
    store.add(entries, Origin::Manual, at).await.unwrap();
    let log = MemoryNotFoundLog::default();
    for path in [
        "/blog/internal-links/",
        "/blog/wp-login.php",
        "/blog/internal-links/",
    ] {
        log.record(path, at).await.unwrap();
    }
    let pages = [
        "/blog/",
        "/blog/why-internal-links-matter/",
        "/blog/robots-txt/",
    ];
    RedirectDesk::new(
        Arc::new(store),
        Arc::new(log),
        Arc::new(StaticPages::new(&pages).unwrap()),
        Arc::new(FixedClock::new(at)),
    )
}

/// Sends a request with `key` to the API over `desk`, answering the status
/// and the body as JSON, or as a JSON string when it is not JSON.
#[cfg(test)]
async fn send(desk: RedirectDesk, key: &str, request: Request<Body>) -> (StatusCode, Value) {
    let hashing = KeyHashing::fast_for_tests();
    let stored = |text: &str, scope| {
        let token: KeyToken = text.parse().unwrap();
        StoredKey::for_integration(&token, vec![scope], &hashing).unwrap()
    };
    let keys = MemoryKeyStore::new(vec![
        stored(SETTINGS, Scope::SettingsWrite),
        stored(POSTS_ONLY, Scope::PostsWrite),
    ]);
    let door = PublishDoor::new(Arc::new(MemoryPostStore::default()), Arc::new(Silent));
    let (router, _) = api::router_with_redirects(
        Arc::new(common::store()),
        Arc::new(door),
        Arc::new(keys),
        desk,
    );
    let (mut parts, body) = request.into_parts();
    let authorization = format!("Khalid {key}").parse().unwrap();
    parts.headers.insert("authorization", authorization);
    let response = router
        .oneshot(Request::from_parts(parts, body))
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    (
        status,
        serde_json::from_str(&text).unwrap_or(Value::String(text)),
    )
}

#[cfg(test)]
fn get(uri: &str) -> Request<Body> {
    Request::get(uri).body(Body::empty()).unwrap()
}

#[tokio::test]
async fn the_list_shows_every_stored_redirect_with_its_columns() {
    let desk = desk(
        "/blog/old-links/,/blog/why-internal-links-matter/,301\n\
         /blog/retired-tool/,,410\n\
         /blog/\\d{4}/(.+),/blog/$1,302,regex\n",
    )
    .await;
    let (status, json) = send(desk, SETTINGS, get("/api/admin/v1/redirects/")).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    let first = &json["redirects"][0];
    assert_eq!(first["from"], "/blog/old-links/");
    assert_eq!(first["to"], "/blog/why-internal-links-matter/");
    assert_eq!(first["status"], 301);
    assert_eq!(first["match"], "exact");
    assert_eq!(first["origin"], "manual");
    assert_eq!(first["created_at"], "2026-11-03T09:00:00Z");
    assert!(first["id"].is_i64(), "{json}");
    assert_eq!(json["redirects"][1]["to"], Value::Null);
    assert_eq!(json["redirects"][1]["status"], 410);
    assert_eq!(json["redirects"][2]["match"], "regex");
    assert_eq!(json["redirects"][2]["to"], "/blog/$1");
}

#[tokio::test]
async fn redirects_need_the_settings_scope() {
    let desk = desk("/blog/sale/,/blog/offers/,302\n").await;
    let (status, json) = send(desk, POSTS_ONLY, get("/api/admin/v1/redirects/")).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{json}");
    assert_eq!(json["rule"], "settings:write");
}

#[tokio::test]
async fn a_site_without_a_redirect_store_lists_none() {
    let (status, json) = send(
        RedirectDesk::none(),
        SETTINGS,
        get("/api/admin/v1/redirects/"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["redirects"], Value::Array(Vec::new()));
}

#[cfg(test)]
fn create(redirect: &Value) -> Request<Body> {
    let body = json!({"redirects": [redirect]});
    Request::post("/api/admin/v1/redirects/")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

#[tokio::test]
async fn a_new_redirect_of_each_status_is_stored_by_hand() {
    let cases = [
        json!({"from": "/blog/a/", "to": "/blog/robots-txt/", "status": 301}),
        json!({"from": "/blog/b/", "to": "https://serpdad.com/", "status": 302}),
        json!({"from": "/blog/c/", "to": "/blog/", "status": 307, "match": "exact"}),
        json!({"from": "/blog/d/", "status": 410}),
        json!({"from": "/blog/e/", "status": 451}),
        json!({"from": "/blog/(\\d+)/", "to": "/blog/post-$1/", "status": 301, "match": "regex"}),
    ];
    for redirect in cases {
        let (status, json) = send(desk("").await, SETTINGS, create(&redirect)).await;
        assert_eq!(status, StatusCode::CREATED, "{redirect} {json}");
        let made = &json["redirects"][0];
        assert_eq!(made["from"], redirect["from"]);
        assert_eq!(made["status"], redirect["status"]);
        assert_eq!(made["origin"], "manual");
        assert_eq!(made["created_at"], "2026-11-03T09:00:00Z");
    }
}

#[tokio::test]
async fn a_redirect_that_cannot_be_followed_is_a_422() {
    let cases = [
        json!({"from": "blog/a/", "to": "/blog/b/", "status": 301}),
        json!({"from": "/blog/a/", "status": 301}),
        json!({"from": "/blog/a/", "to": "/blog/b/", "status": 308}),
        json!({"from": "/blog/a/", "to": "/blog/b/", "status": 301, "match": "glob"}),
        json!({"from": "/blog/(/", "to": "/blog/b/", "status": 301, "match": "regex"}),
    ];
    for redirect in cases {
        let (status, json) = send(desk("").await, SETTINGS, create(&redirect)).await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{redirect} {json}"
        );
        let detail = json["detail"].as_str().unwrap();
        assert!(detail.contains("cannot be stored"), "{json}");
    }
}

#[tokio::test]
async fn a_loop_or_a_taken_address_is_refused_and_nothing_stored() {
    let stored = "/blog/a/,/blog/b/,301\n";
    let back = json!({"from": "/blog/b/", "to": "/blog/a/", "status": 301});
    let (status, json) = send(desk(stored).await, SETTINGS, create(&back)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{json}");
    assert!(json["detail"].as_str().unwrap().contains("loop"), "{json}");

    let again = json!({"from": "/blog/a/", "to": "/blog/c/", "status": 302});
    let (status, json) = send(desk(stored).await, SETTINGS, create(&again)).await;
    assert_eq!(status, StatusCode::CONFLICT, "{json}");
    assert!(
        json["detail"].as_str().unwrap().contains("/blog/a/"),
        "{json}"
    );
}

#[tokio::test]
async fn a_redirect_is_removed_by_its_identity() {
    let desk = desk("/blog/a/,/blog/b/,301\n").await;
    let (_, json) = send(desk.clone(), SETTINGS, get("/api/admin/v1/redirects/")).await;
    let uri = format!("/api/admin/v1/redirects/{}/", json["redirects"][0]["id"]);
    let delete = || Request::delete(uri.as_str()).body(Body::empty()).unwrap();
    let (status, _) = send(desk.clone(), SETTINGS, delete()).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, json) = send(desk.clone(), SETTINGS, delete()).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{json}");
    let (_, json) = send(desk.clone(), SETTINGS, get("/api/admin/v1/redirects/")).await;
    assert_eq!(json["redirects"], Value::Array(Vec::new()));

    let words = Request::delete("/api/admin/v1/redirects/old-links/").body(Body::empty());
    let (status, json) = send(desk, SETTINGS, words.unwrap()).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{json}");
    assert!(
        json["detail"].as_str().unwrap().contains("No redirect"),
        "{json}"
    );
}

#[tokio::test]
async fn the_debug_view_names_the_redirect_that_matched_and_each_hop() {
    let desk = desk(
        "/blog/\\d{4}/(.+),/blog/$1,301,regex\n\
         /blog/old-links/,/blog/internal-links/,301\n\
         /blog/internal-links/,/blog/why-internal-links-matter/,302\n",
    )
    .await;
    let (_, list) = send(desk.clone(), SETTINGS, get("/api/admin/v1/redirects/")).await;
    let id = |n: usize| list["redirects"][n]["id"].clone();
    let uri = "/api/admin/v1/redirects/explain/?path=/blog/2019/old-links/";
    let (status, json) = send(desk, SETTINGS, get(uri)).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["path"], "/blog/2019/old-links/");
    assert_eq!(json["matched"], id(0));
    let hops: Vec<_> = json["hops"]
        .as_array()
        .unwrap()
        .iter()
        .map(|h| h["redirect"].clone())
        .collect();
    assert_eq!(hops, [id(0), id(1), id(2)]);
    assert_eq!(json["hops"][2]["status"], 302);
    assert_eq!(json["status"], 302);
    assert_eq!(json["to"], "/blog/why-internal-links-matter/");
    let text = json["text"].as_str().unwrap();
    assert!(text.contains("the pattern /blog/\\d{4}/(.+)."), "{text}");
    assert!(
        text.ends_with(
            "A visitor gets one step: moves for now (302) to /blog/why-internal-links-matter/.\n"
        ),
        "{text}"
    );
}

#[tokio::test]
async fn the_debug_view_says_when_nothing_matches_and_refuses_a_bad_path() {
    let desk = desk("/blog/a/,/blog/b/,301\n").await;
    let uri = "/api/admin/v1/redirects/explain/?path=/blog/robots-txt/";
    let (status, json) = send(desk.clone(), SETTINGS, get(uri)).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(
        (&json["matched"], &json["status"]),
        (&Value::Null, &Value::Null)
    );
    assert_eq!(json["hops"], Value::Array(Vec::new()));
    assert!(
        json["text"]
            .as_str()
            .unwrap()
            .contains("matches no redirect rule"),
        "{json}"
    );

    let uri = "/api/admin/v1/redirects/explain/?path=blog/no-slash/";
    let (status, json) = send(desk, SETTINGS, get(uri)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{json}");
    assert_eq!(json["rule"], "path");
}

#[cfg(test)]
fn import(csv: &str) -> Request<Body> {
    Request::post("/api/admin/v1/redirects/import/")
        .header("content-type", "text/csv")
        .body(Body::from(csv.to_owned()))
        .unwrap()
}

/// The list the importers write, one row of each kind.
const IMPORTED: &str = "from,to,status,match\n\
    /blog/2019/05/old-links/,/blog/old-links/,301,exact\n\
    /blog/retired-tool/,,410,exact\n\
    /blog/tmp/.*,,410,regex\n";

#[tokio::test]
async fn an_imported_list_is_stored_whole_and_exported_the_same() {
    let desk = desk("/blog/old-links/,/blog/why-internal-links-matter/,301\n").await;
    let (status, json) = send(desk.clone(), SETTINGS, import(IMPORTED)).await;
    assert_eq!(status, StatusCode::CREATED, "{json}");
    assert_eq!(json["redirects"].as_array().unwrap().len(), 3);
    assert_eq!(json["redirects"][0]["origin"], "import");

    let export = get("/api/admin/v1/redirects/export/");
    let (status, csv) = send(desk.clone(), SETTINGS, export).await;
    assert_eq!(status, StatusCode::OK);
    let csv = csv.as_str().unwrap();
    assert!(
        csv.starts_with("from,to,status,match\n/blog/old-links/,"),
        "{csv}"
    );
    assert!(
        csv.ends_with(IMPORTED.trim_start_matches("from,to,status,match\n")),
        "{csv}"
    );

    let uri = "/api/admin/v1/redirects/explain/?path=/blog/2019/05/old-links/";
    let (_, json) = send(desk, SETTINGS, get(uri)).await;
    assert_eq!(json["to"], "/blog/why-internal-links-matter/", "{json}");
}

#[tokio::test]
async fn a_list_with_a_bad_row_a_loop_or_a_taken_address_stores_nothing() {
    let stored = "/blog/a/,/blog/b/,301\n";
    let lists = [
        (
            "from,to,status,match\n/blog/c/,/blog/d/,301\n/blog/e/,,301\n",
            StatusCode::UNPROCESSABLE_ENTITY,
            "line 3",
        ),
        (
            "/blog/c/,/blog/d/,301\n",
            StatusCode::UNPROCESSABLE_ENTITY,
            "line 1",
        ),
        (
            "from,to,status\n/blog/b/,/blog/a/,301\n",
            StatusCode::UNPROCESSABLE_ENTITY,
            "loop",
        ),
        (
            "from,to,status\n/blog/c/,/blog/d/,301\n/blog/a/,/blog/e/,302\n",
            StatusCode::CONFLICT,
            "/blog/a/",
        ),
    ];
    for (csv, want, says) in lists {
        let desk = desk(stored).await;
        let (status, json) = send(desk.clone(), SETTINGS, import(csv)).await;
        assert_eq!(status, want, "{csv} {json}");
        assert!(
            json["detail"].as_str().unwrap().contains(says),
            "{csv} {json}"
        );
        let (_, json) = send(desk, SETTINGS, get("/api/admin/v1/redirects/")).await;
        assert_eq!(json["redirects"].as_array().unwrap().len(), 1, "{csv}");
    }
}

#[tokio::test]
async fn the_log_lists_misses_most_hit_first_with_the_nearest_page() {
    let (status, json) = send(desk("").await, SETTINGS, get("/api/admin/v1/not-found/")).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    let first = &json["misses"][0];
    assert_eq!(first["path"], "/blog/internal-links/");
    assert_eq!(first["hits"], 2);
    assert_eq!(first["last_seen"], "2026-11-03T09:00:00Z");
    assert_eq!(first["nearest"], "/blog/why-internal-links-matter/");
    assert_eq!(json["misses"][1]["nearest"], Value::Null);
    let (_, json) = send(
        desk("").await,
        SETTINGS,
        get("/api/admin/v1/not-found/?limit=1"),
    )
    .await;
    assert_eq!(json["misses"].as_array().unwrap().len(), 1);
    let (status, json) = send(
        desk("").await,
        SETTINGS,
        get("/api/admin/v1/not-found/?limit=0"),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{json}");
}
