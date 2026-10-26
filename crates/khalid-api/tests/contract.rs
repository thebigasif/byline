//! The contract test (RULES.md BL-API-4, KH-API-4): it fails when the
//! routes and the `OpenAPI` document disagree.
//!
//! Every path and method in the document must reach a handler, which
//! answers with JSON or a problem, never the router's bare 404 or 405. And
//! every resource name in `docs/standards/api.md` is tried under both API
//! bases: one that answers must be in the document.

mod common;

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use khalid_api::api;
use khalid_api::keys::{KeyHashing, KeyToken, MemoryKeyStore, Scope, StoredKey};
use khalid_api::post_store::MemoryPostStore;
use serde_json::Value;
use tower::ServiceExt;

/// Keys made up for these tests: a content key, and an integration key
/// with every scope. They open nothing real.
const KEY: &str =
    "kc_0123456789abcdef_00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";
const ADMIN: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";

/// The resources `docs/standards/api.md` names.
const RESOURCES: [&str; 17] = [
    "posts",
    "pages",
    "authors",
    "tags",
    "categories",
    "collections",
    "glossary",
    "case-studies",
    "changelog",
    "media",
    "redirects",
    "snippets",
    "forms",
    "subscribers",
    "webhooks",
    "settings",
    "site",
];

/// The methods a route could answer.
const METHODS: [&str; 5] = ["get", "post", "put", "patch", "delete"];

#[cfg(test)]
fn api() -> (Router, Value) {
    let token: KeyToken = KEY.parse().unwrap();
    let admin: KeyToken = ADMIN.parse().unwrap();
    let hashing = KeyHashing::fast_for_tests();
    let all = StoredKey::for_integration(&admin, vec![Scope::AdminAll], &hashing).unwrap();
    let keys = MemoryKeyStore::new(vec![StoredKey::for_token(&token), all]);
    let posts = Arc::new(MemoryPostStore::default());
    let (router, document) = api::router(Arc::new(common::store()), posts, Arc::new(keys));
    (router, serde_json::to_value(document).unwrap())
}

/// A valid key for the API `path` belongs to, so a key layer never answers
/// in a handler's place.
#[cfg(test)]
fn authorization(path: &str) -> String {
    if path.starts_with("/api/admin/") {
        format!("Khalid {ADMIN}")
    } else {
        format!("Khalid-Content {KEY}")
    }
}

/// Whether a request reached a handler: the router's own refusals carry
/// no media type.
#[cfg(test)]
async fn reaches_a_handler(router: &Router, method: &str, path: &str) -> bool {
    let request = Request::builder()
        .method(Method::from_bytes(method.to_uppercase().as_bytes()).unwrap())
        .uri(path)
        .header("authorization", authorization(path))
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .unwrap();
    let response = router.clone().oneshot(request).await.unwrap();
    let matched = !matches!(
        response.status(),
        StatusCode::NOT_FOUND | StatusCode::METHOD_NOT_ALLOWED
    );
    matched || response.headers().contains_key("content-type")
}

/// A concrete address for a documented path.
#[cfg(test)]
fn concrete(path: &str) -> String {
    path.replace("{id}", "contract-id")
        .replace("{slug}", "contract-slug")
}

#[tokio::test]
async fn every_documented_route_reaches_a_handler() {
    let (router, document) = api();
    let paths = document["paths"].as_object().unwrap();
    assert!(!paths.is_empty());
    for (path, item) in paths {
        for method in METHODS {
            let documented = item.get(method).is_some();
            let reached = reaches_a_handler(&router, method, &concrete(path)).await;
            assert_eq!(
                reached, documented,
                "{method} {path}: documented {documented}, routed {reached}"
            );
        }
    }
}

#[tokio::test]
async fn every_route_that_answers_is_documented() {
    let (router, document) = api();
    let paths = document["paths"].as_object().unwrap();
    for base in ["/api/content/v1", "/api/admin/v1"] {
        for resource in RESOURCES {
            for suffix in ["/", "/contract-id/", "/slug/contract-slug/"] {
                let address = format!("{base}/{resource}{suffix}");
                for method in METHODS {
                    if reaches_a_handler(&router, method, &address).await {
                        let listed = paths.iter().any(|(path, item)| {
                            concrete(path) == address && item.get(method).is_some()
                        });
                        assert!(
                            listed,
                            "{method} {address} answers but is not in the document"
                        );
                    }
                }
            }
        }
    }
}
