//! `ETag` headers, cache headers and `Accept-Version` on the Content API
//! (`docs/standards/api.md`, RULES.md BL-API-4).

mod common;

use std::sync::Arc;

use axum::body::{Body, Bytes, to_bytes};
use axum::http::{HeaderMap, Request, StatusCode};
use khalid_api::content_api;
use khalid_api::keys::{KeyToken, MemoryKeyStore, StoredKey};
use serde_json::Value;
use tower::ServiceExt;

/// A content key made up for these tests. It opens nothing real.
const KEY: &str =
    "kc_0123456789abcdef_00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";

#[cfg(test)]
async fn send(uri: &str, headers: &[(&str, &str)]) -> (StatusCode, HeaderMap, Bytes) {
    let token: KeyToken = KEY.parse().unwrap();
    let keys = MemoryKeyStore::new(vec![StoredKey::for_token(&token)]);
    let (router, _) = content_api::router(Arc::new(common::store()), Arc::new(keys));
    let mut request = Request::get(uri).header("authorization", format!("Khalid-Content {KEY}"));
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let response = router
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let (parts, body) = response.into_parts();
    (
        parts.status,
        parts.headers,
        to_bytes(body, 1024 * 1024).await.unwrap(),
    )
}

#[tokio::test]
async fn a_list_carries_a_stable_etag_and_cache_headers() {
    let (status, headers, _) = send("/api/content/v1/posts/", &[]).await;
    assert_eq!(status, StatusCode::OK);
    let etag = headers["etag"].to_str().unwrap().to_owned();
    assert!(
        etag.starts_with('"') && etag.ends_with('"') && etag.len() > 10,
        "{etag}"
    );
    assert_eq!(
        headers["cache-control"],
        "public, max-age=0, must-revalidate"
    );
    assert_eq!(headers["vary"], "authorization, accept-version");
    let (_, again, _) = send("/api/content/v1/posts/", &[]).await;
    assert_eq!(again["etag"].to_str().unwrap(), etag);
    let (_, other, _) = send("/api/content/v1/posts/?limit=1", &[]).await;
    assert_ne!(other["etag"].to_str().unwrap(), etag);
}

#[tokio::test]
async fn a_matching_if_none_match_gets_304_with_no_body() {
    let (_, headers, _) = send("/api/content/v1/posts/slug/fixing-slow-pages/", &[]).await;
    let etag = headers["etag"].to_str().unwrap().to_owned();
    for value in [etag.as_str(), &format!("\"other\", {etag}"), "*"] {
        let (status, headers, body) = send(
            "/api/content/v1/posts/slug/fixing-slow-pages/",
            &[("if-none-match", value)],
        )
        .await;
        assert_eq!(status, StatusCode::NOT_MODIFIED, "{value}");
        assert_eq!(headers["etag"].to_str().unwrap(), etag);
        assert!(body.is_empty());
    }
    let (status, _, _) = send(
        "/api/content/v1/posts/slug/fixing-slow-pages/",
        &[("if-none-match", "\"stale\"")],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn errors_carry_no_etag() {
    let (status, headers, _) = send("/api/content/v1/posts/?limit=0", &[]).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(headers.get("etag"), None);
}

#[tokio::test]
async fn every_answer_names_the_version_served() {
    for uri in ["/api/content/v1/posts/", "/api/content/v1/posts/?limit=0"] {
        let (_, headers, _) = send(uri, &[]).await;
        assert_eq!(headers["content-version"], "v1.0", "{uri}");
    }
}

#[tokio::test]
async fn accept_version_up_to_this_one_is_served() {
    for version in ["v1.0", "v1", "V1.0"] {
        let (status, headers, _) =
            send("/api/content/v1/posts/", &[("accept-version", version)]).await;
        assert_eq!(status, StatusCode::OK, "{version}");
        assert_eq!(headers["content-version"], "v1.0");
    }
}

#[tokio::test]
async fn a_newer_or_unreadable_accept_version_is_a_problem() {
    let cases = [
        ("v1.3", StatusCode::NOT_ACCEPTABLE, "version-not-supported"),
        ("v2.0", StatusCode::NOT_ACCEPTABLE, "version-not-supported"),
        ("1.0", StatusCode::BAD_REQUEST, "invalid-version"),
        ("v1.x", StatusCode::BAD_REQUEST, "invalid-version"),
    ];
    for (version, status, kind) in cases {
        let (got, headers, body) =
            send("/api/content/v1/posts/", &[("accept-version", version)]).await;
        assert_eq!(got, status, "{version}");
        assert_eq!(headers["content-type"], "application/problem+json");
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            json["type"],
            format!("https://khalid.dev/problems/{kind}"),
            "{version}"
        );
    }
}

#[tokio::test]
async fn every_operation_documents_accept_version() {
    let (_, _, body) = send("/api/openapi.json", &[]).await;
    let json: Value = serde_json::from_slice(&body).unwrap();
    let paths = json["paths"].as_object().unwrap();
    for (path, item) in paths {
        let parameters = item["get"]["parameters"].as_array().unwrap();
        let header = parameters.iter().find(|p| p["name"] == "Accept-Version");
        assert_eq!(
            header.map(|p| p["in"].clone()),
            Some(Value::from("header")),
            "{path}"
        );
    }
}
