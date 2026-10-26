//! The Content API over HTTP: the posts routes and the `OpenAPI` document,
//! with the demo posts behind them.

mod common;

use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{HeaderMap, Request, StatusCode};
use khalid_api::content_api;
use khalid_api::keys::{KeyToken, MemoryKeyStore, StoredKey};
use serde_json::Value;
use tower::ServiceExt;

/// A content key made up for these tests. It opens nothing real.
const KEY: &str =
    "kc_0123456789abcdef_00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";

#[cfg(test)]
async fn send(uri: &str, authorization: Option<&str>) -> (StatusCode, HeaderMap, Value) {
    let token: KeyToken = KEY.parse().unwrap();
    let keys = MemoryKeyStore::new(vec![StoredKey::for_token(&token)]);
    let (router, _) = content_api::router(Arc::new(common::store()), Arc::new(keys));
    let mut request = Request::get(uri);
    if let Some(value) = authorization {
        request = request.header("authorization", value);
    }
    let response = router
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let (parts, body) = response.into_parts();
    let body = to_bytes(body, 1024 * 1024).await.unwrap();
    (
        parts.status,
        parts.headers,
        serde_json::from_slice(&body).unwrap(),
    )
}

/// A request with the content key in the `Authorization` header.
#[cfg(test)]
async fn get(uri: &str) -> (StatusCode, String, Value) {
    let (status, headers, json) = send(uri, Some(&format!("Khalid-Content {KEY}"))).await;
    let media = headers
        .get("content-type")
        .map(|value| value.to_str().unwrap().to_owned());
    (status, media.unwrap_or_default(), json)
}

#[tokio::test]
async fn the_content_key_may_also_come_in_the_query() {
    let (status, _, json) = send(&format!("/api/content/v1/posts/?key={KEY}&limit=1"), None).await;
    assert_eq!(
        (status, json["posts"].as_array().map(Vec::len)),
        (StatusCode::OK, Some(1))
    );
}

#[tokio::test]
async fn a_missing_or_wrong_key_is_a_401_problem() {
    let wrong = KEY.replace('f', "e");
    let cases = [
        ("/api/content/v1/posts/".to_owned(), None),
        (
            "/api/content/v1/posts/".to_owned(),
            Some(format!("Khalid-Content {wrong}")),
        ),
        (
            "/api/content/v1/posts/".to_owned(),
            Some(format!("Bearer {KEY}")),
        ),
        (format!("/api/content/v1/posts/?key={wrong}"), None),
        (
            "/api/content/v1/posts/slug/fixing-slow-pages/".to_owned(),
            None,
        ),
    ];
    for (uri, authorization) in cases {
        let (status, headers, json) = send(&uri, authorization.as_deref()).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{uri} {authorization:?}");
        assert_eq!(headers["www-authenticate"], "Khalid-Content", "{uri}");
        assert_eq!(headers["content-type"], "application/problem+json", "{uri}");
        assert_eq!(
            json["type"], "https://khalid.dev/problems/unauthorized",
            "{uri}"
        );
        assert!(
            !json.to_string().contains(KEY.get(3..19).unwrap()),
            "{json}"
        );
    }
}

#[tokio::test]
async fn the_openapi_document_needs_no_key_and_names_the_schemes() {
    let (status, _, json) = send("/api/openapi.json", None).await;
    assert_eq!(status, StatusCode::OK);
    let schemes = &json["components"]["securitySchemes"];
    assert_eq!(schemes["content_key"]["in"], "header");
    assert_eq!(schemes["content_key_query"]["in"], "query");
    assert_eq!(schemes["content_key_query"]["name"], "key");
    let security = &json["paths"]["/api/content/v1/posts/"]["get"]["security"];
    assert_eq!(security.as_array().map(Vec::len), Some(2));
}

#[cfg(test)]
fn slugs(json: &Value) -> Vec<&str> {
    let posts = json["posts"].as_array().unwrap();
    posts
        .iter()
        .map(|post| post["slug"].as_str().unwrap())
        .collect()
}

#[tokio::test]
async fn the_demo_posts_come_back_newest_first_with_pagination() {
    let (status, media, json) = get("/api/content/v1/posts/").await;
    assert_eq!(
        (status, media.as_str()),
        (StatusCode::OK, "application/json")
    );
    assert_eq!(slugs(&json).first(), Some(&"planning-a-topic-cluster"));
    assert_eq!(slugs(&json).len(), 5);
    assert_eq!(json["meta"]["pagination"]["total"], 5);
    assert_eq!(
        json["posts"][0]["html"],
        "<p>Planning a topic cluster in an afternoon.</p>"
    );
    assert_eq!(json["posts"][0].get("plaintext"), None);
}

#[tokio::test]
async fn the_list_parameters_apply() {
    let (_, _, json) = get("/api/content/v1/posts/?filter=tag:links&limit=1&page=2").await;
    assert_eq!(slugs(&json), ["why-internal-links-matter"]);
    assert_eq!(json["meta"]["pagination"]["pages"], 2);
    let (_, _, json) = get("/api/content/v1/posts/?formats=plaintext&order=title%20asc").await;
    assert_eq!(slugs(&json).first(), Some(&"fixing-slow-pages"));
    assert_eq!(json["posts"][0].get("html"), None);
    assert_eq!(
        json["posts"][0]["plaintext"],
        "Fixing slow pages, starting with the biggest win."
    );
}

#[tokio::test]
async fn a_bad_list_parameter_is_a_400_problem() {
    for (query, rule) in [
        ("limit=0", "limit"),
        ("filter=tag:", "filter"),
        ("filter=colour:red", "filter"),
        ("order=colour%20asc", "order"),
    ] {
        let (status, media, json) = get(&format!("/api/content/v1/posts/?{query}")).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{query}");
        assert_eq!(media, "application/problem+json", "{query}");
        assert_eq!(json["rule"], rule, "{query}");
    }
    let (_, _, json) = get("/api/content/v1/posts/?filter=colour:red").await;
    assert_eq!(
        json["detail"],
        "The filter uses colour, which is not a field here. The list cannot be sent until it is fixed."
    );
}

#[tokio::test]
async fn one_post_by_slug_or_id() {
    let (status, _, json) = get("/api/content/v1/posts/slug/fixing-slow-pages/").await;
    assert_eq!(
        (status, slugs(&json)),
        (StatusCode::OK, vec!["fixing-slow-pages"])
    );
    assert_eq!(json.get("meta"), None);
    let (status, _, json) = get("/api/content/v1/posts/what-a-sitemap-does-en/").await;
    assert_eq!(
        (status, slugs(&json)),
        (StatusCode::OK, vec!["what-a-sitemap-does"])
    );
}

#[tokio::test]
async fn a_missing_post_is_a_404_problem() {
    for uri in [
        "/api/content/v1/posts/slug/no-such-post/",
        "/api/content/v1/posts/slug/Not%20A%20Slug/",
        "/api/content/v1/posts/no-such-id/",
    ] {
        let (status, media, json) = get(uri).await;
        let want = (StatusCode::NOT_FOUND, "application/problem+json");
        assert_eq!((status, media.as_str()), want, "{uri}");
        assert_eq!(json["instance"], uri, "{uri}");
    }
}

#[tokio::test]
async fn the_openapi_document_lists_the_routes() {
    let (status, _, json) = get("/api/openapi.json").await;
    assert_eq!(status, StatusCode::OK);
    let paths = json["paths"].as_object().unwrap();
    let mut listed: Vec<&str> = paths.keys().map(String::as_str).collect();
    listed.sort_unstable();
    let routes = [
        "/api/content/v1/posts/",
        "/api/content/v1/posts/slug/{slug}/",
        "/api/content/v1/posts/{id}/",
    ];
    assert_eq!(listed, routes);
    let parameters = json["paths"]["/api/content/v1/posts/"]["get"]["parameters"]
        .as_array()
        .unwrap();
    let names: Vec<&str> = parameters
        .iter()
        .map(|parameter| parameter["name"].as_str().unwrap())
        .collect();
    let want = [
        "filter",
        "include",
        "fields",
        "order",
        "formats",
        "limit",
        "page",
        "Accept-Version",
    ];
    assert_eq!(names, want);
    assert!(json["components"]["schemas"]["Post"].is_object());
    assert!(json["components"]["schemas"]["Problem"].is_object());
}
