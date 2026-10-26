//! The Content API's authors, tags, categories, collections and menus over
//! HTTP: each lists, filters and reads one entry by identity or slug, like
//! the posts. And the site's settings, which are one entry.

mod common;

use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use khalid_api::content_api;
use khalid_api::content_store::MemoryContentStore;
use khalid_api::keys::{KeyToken, MemoryKeyStore, StoredKey};
use serde_json::Value;
use tower::ServiceExt;

/// A content key made up for these tests. It opens nothing real.
const KEY: &str =
    "kc_0123456789abcdef_00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";

/// Sends `uri` to the demo store with the content key and returns the
/// status, the media type and the body.
#[cfg(test)]
async fn get(uri: &str) -> (StatusCode, String, Value) {
    get_from(common::store(), uri).await
}

/// Sends `uri` to `store` with the content key.
#[cfg(test)]
async fn get_from(store: MemoryContentStore, uri: &str) -> (StatusCode, String, Value) {
    let token: KeyToken = KEY.parse().unwrap();
    let keys = MemoryKeyStore::new(vec![StoredKey::for_token(&token)]);
    let (router, _) = content_api::router(Arc::new(store), Arc::new(keys));
    let request = Request::get(uri)
        .header("authorization", format!("Khalid-Content {KEY}"))
        .body(Body::empty())
        .unwrap();
    let response = router.oneshot(request).await.unwrap();
    let (parts, body) = response.into_parts();
    let media = parts
        .headers
        .get("content-type")
        .map_or(String::new(), |v| v.to_str().unwrap().to_owned());
    let body = to_bytes(body, 1024 * 1024).await.unwrap();
    (parts.status, media, serde_json::from_slice(&body).unwrap())
}

/// The `field` of every entry under `key`.
#[cfg(test)]
fn each<'a>(json: &'a Value, key: &str, field: &str) -> Vec<&'a str> {
    json[key]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry[field].as_str().unwrap())
        .collect()
}

#[tokio::test]
async fn the_authors_list_in_the_order_the_store_gives() {
    let (status, media, json) = get("/api/content/v1/authors/").await;
    assert_eq!(
        (status, media.as_str()),
        (StatusCode::OK, "application/json")
    );
    assert_eq!(
        each(&json, "authors", "id"),
        ["en.asif", "en.lena-vogt", "de.asif"]
    );
    assert_eq!(json["meta"]["pagination"]["total"], 3);
    let asif = &json["authors"][0];
    assert_eq!(asif["name"], "Asif");
    assert_eq!(asif["role"], "Founder of SerpDad");
    assert_eq!(asif["url"], "https://example.com/blog/authors/asif/");
    assert_eq!(asif["same_as"][0], "https://serpdad.com/");
    assert_eq!(json["authors"][1]["role"], Value::Null);
}

#[tokio::test]
async fn the_settings_are_one_entry_or_a_404() {
    let (status, media, json) = get("/api/content/v1/settings/").await;
    assert_eq!(
        (status, media.as_str()),
        (StatusCode::OK, "application/json")
    );
    assert_eq!(
        json,
        serde_json::json!({"settings": [{
            "title": "Field Notes on SEO",
            "url": "https://example.com/blog/",
            "default_language": "en",
            "languages": ["en", "de"],
        }]})
    );
    let empty = MemoryContentStore::new(Vec::new());
    let (status, media, json) = get_from(empty, "/api/content/v1/settings/").await;
    assert_eq!(
        (status, media.as_str()),
        (StatusCode::NOT_FOUND, "application/problem+json")
    );
    assert_eq!(json["instance"], "/api/content/v1/settings/");
}

#[tokio::test]
async fn collections_and_menus_carry_their_own_fields() {
    let (_, _, json) = get("/api/content/v1/collections/en.posts/").await;
    let posts = &json["collections"][0];
    assert_eq!(posts["slug"], "posts");
    assert_eq!(posts["title"], "Field notes on SEO");
    assert_eq!(posts["description"], "Plain posts on search.");
    assert_eq!(json["collections"].as_array().map(Vec::len), Some(1));
    let (_, _, json) = get("/api/content/v1/navigation/").await;
    assert_eq!(
        each(&json, "navigation", "id"),
        ["en.primary", "en.footer", "de.primary"]
    );
    let primary = &json["navigation"][0];
    assert_eq!(primary["links"][0]["label"], "About");
    assert_eq!(primary["links"][0]["url"], "/blog/about/");
    assert_eq!(json["navigation"][1]["links"], serde_json::json!([]));
    let (status, _, json) = get("/api/content/v1/collections/?filter=name:posts").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        json["fix"],
        "Use one of these fields: id, slug, title, language."
    );
}

#[tokio::test]
async fn the_list_parameters_apply_to_every_site_list() {
    let cases = [
        ("authors/?filter=language:de", "authors", vec!["de.asif"]),
        (
            "authors/?order=name%20desc&limit=1",
            "authors",
            vec!["en.lena-vogt"],
        ),
        (
            "tags/?filter=slug:[seo,speed]",
            "tags",
            vec!["en.seo", "en.speed"],
        ),
        (
            "categories/?filter=name:~'seo'",
            "categories",
            vec!["en.on-page-seo", "de.on-page-seo"],
        ),
        (
            "collections/?filter=language:en",
            "collections",
            vec!["en.posts", "en.glossary"],
        ),
        (
            "navigation/?filter=slug:footer",
            "navigation",
            vec!["en.footer"],
        ),
    ];
    for (query, key, want) in cases {
        let (status, _, json) = get(&format!("/api/content/v1/{query}")).await;
        assert_eq!(status, StatusCode::OK, "{query}: {json}");
        assert_eq!(each(&json, key, "id"), want, "{query}");
    }
}

#[tokio::test]
async fn one_entry_by_identity_or_slug() {
    let cases = [
        ("authors/slug/lena-vogt/", "authors", "en.lena-vogt"),
        ("authors/de.asif/", "authors", "de.asif"),
        ("tags/slug/links/", "tags", "en.links"),
        ("categories/de.on-page-seo/", "categories", "de.on-page-seo"),
        ("collections/slug/glossary/", "collections", "en.glossary"),
        ("navigation/de.primary/", "navigation", "de.primary"),
    ];
    for (path, key, id) in cases {
        let (status, _, json) = get(&format!("/api/content/v1/{path}")).await;
        assert_eq!(status, StatusCode::OK, "{path}");
        assert_eq!(each(&json, key, "id"), [id], "{path}");
        assert_eq!(json.get("meta"), None, "{path}");
    }
}

#[tokio::test]
async fn a_missing_entry_is_a_404_and_a_wrong_field_a_400() {
    for path in [
        "authors/slug/nobody/",
        "tags/en.nothing/",
        "categories/slug/Not%20A%20Slug/",
        "collections/de.glossary/",
        "navigation/slug/sidebar/",
    ] {
        let uri = format!("/api/content/v1/{path}");
        let (status, media, json) = get(&uri).await;
        assert_eq!(
            (status, media.as_str()),
            (StatusCode::NOT_FOUND, "application/problem+json"),
            "{path}"
        );
        assert_eq!(json["instance"], uri, "{path}");
    }
    let (status, _, json) = get("/api/content/v1/authors/?filter=published_at:null").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(json["rule"], "filter");
    assert_eq!(
        json["fix"],
        "Use one of these fields: id, slug, name, language."
    );
}
