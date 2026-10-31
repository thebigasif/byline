//! Rate limits on both APIs (RULES.md BL-SEC-5): a busy key, and an
//! address that keeps sending wrong keys, get a 429 problem with
//! `Retry-After`.

mod common;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode, header};
use khalid_api::api;
use khalid_api::api_limits::ApiLimits;
use khalid_api::keys::{KeyHashing, KeyToken, MemoryKeyStore, Scope, StoredKey};
use khalid_api::post_store::MemoryPostStore;
use serde_json::Value;
use tower::ServiceExt;

/// Keys made up for these tests. They open nothing real.
const WRITER: &str =
    "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
const CONTENT: &str =
    "kc_0123456789abcdef_00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";
/// The content key's id with another secret of the same length.
const WRONG_CONTENT: &str =
    "kc_0123456789abcdef_ffeeddccbbaa99887766554433221100ffeeddccbbaa99887766554433221100";
/// The integration key's id with another secret of the same length.
const WRONG_WRITER: &str =
    "ki_1111222233334444_0000111122223333444455556666777788889999aaaabbbbccccddddeeeeffff";

const POSTS: &str = "/api/content/v1/posts/?limit=1";
const DRAFTS: &str = "/api/admin/v1/posts/";

#[cfg(test)]
fn router() -> Router {
    let hashing = KeyHashing::fast_for_tests();
    let token = |text: &str| text.parse::<KeyToken>().unwrap();
    let keys = MemoryKeyStore::new(vec![
        StoredKey::for_integration(&token(WRITER), vec![Scope::PostsWrite], &hashing).unwrap(),
        StoredKey::for_token(&token(CONTENT)),
    ]);
    let posts = MemoryPostStore::new(Vec::new());
    let (router, _) = api::router(Arc::new(common::store()), Arc::new(posts), Arc::new(keys));
    router
}

/// Sends `GET uri` from `peer` with `Authorization: authorization`, and
/// returns the status, the `Retry-After` header and the body.
#[cfg(test)]
async fn send(
    router: &Router,
    uri: &str,
    peer: &str,
    authorization: Option<&str>,
) -> (StatusCode, Option<String>, Value) {
    let mut request = Request::get(uri);
    if let Some(value) = authorization {
        request = request.header(header::AUTHORIZATION, value);
    }
    let mut request = request.body(Body::empty()).unwrap();
    let peer: SocketAddr = peer.parse().unwrap();
    request.extensions_mut().insert(ConnectInfo(peer));
    let response = router.clone().oneshot(request).await.unwrap();
    let (parts, body) = response.into_parts();
    let retry_after = parts
        .headers
        .get(header::RETRY_AFTER)
        .map(|value| value.to_str().unwrap().to_owned());
    let body = to_bytes(body, 1024 * 1024).await.unwrap();
    let json = serde_json::from_slice(&body).unwrap_or(Value::Null);
    (parts.status, retry_after, json)
}

#[cfg(test)]
fn content(key: &str) -> String {
    format!("Khalid-Content {key}")
}

#[cfg(test)]
fn admin(key: &str) -> String {
    format!("Khalid {key}")
}

#[tokio::test]
async fn an_address_that_sends_wrong_content_keys_is_paused_with_a_429() {
    let router = router();
    let peer = "203.0.113.7:5000";
    for _ in 0..ApiLimits::REFUSED_PER_MINUTE {
        let (status, ..) = send(&router, POSTS, peer, Some(&content(WRONG_CONTENT))).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
    let (status, retry_after, json) = send(&router, POSTS, peer, Some(&content(CONTENT))).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS, "{json}");
    let seconds: u64 = retry_after.unwrap().parse().unwrap();
    assert!((1..=60).contains(&seconds), "{seconds}");
    assert_eq!(
        json["type"],
        "https://khalid.dev/problems/too-many-requests"
    );
    assert_eq!(json["status"], 429);
    assert!(json["fix"].as_str().unwrap().contains("seconds"), "{json}");

    let (status, ..) = send(&router, POSTS, "198.51.100.1:5000", Some(&content(CONTENT))).await;
    assert_eq!(status, StatusCode::OK, "another address is not paused");
}

#[tokio::test]
async fn an_address_that_sends_wrong_admin_keys_is_paused_with_a_429() {
    let router = router();
    let peer = "[2001:db8::1]:5000";
    for _ in 0..ApiLimits::REFUSED_PER_MINUTE {
        let (status, ..) = send(&router, DRAFTS, peer, Some(&admin(WRONG_WRITER))).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
    // Another address in the same /64 network counts as the same one.
    let (status, retry_after, _) =
        send(&router, DRAFTS, "[2001:db8::2]:5000", Some(&admin(WRITER))).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert!(retry_after.is_some());
}

#[tokio::test]
async fn a_request_with_no_key_is_never_counted() {
    let router = router();
    let peer = "203.0.113.8:5000";
    for _ in 0..=ApiLimits::REFUSED_PER_MINUTE {
        let (status, ..) = send(&router, POSTS, peer, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
    let (status, ..) = send(&router, POSTS, peer, Some(&content(CONTENT))).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn a_busy_admin_key_gets_a_429_with_retry_after() {
    let router = router();
    let peer = "203.0.113.9:5000";
    let mut limited = None;
    // Twice the limit: the key earns back at most one request in each
    // tenth of a second, far fewer than this loop sends.
    for _ in 0..2 * ApiLimits::ADMIN_PER_MINUTE {
        let answer = send(&router, DRAFTS, peer, Some(&admin(WRITER))).await;
        if answer.0 == StatusCode::TOO_MANY_REQUESTS {
            limited = Some(answer);
            break;
        }
        assert_eq!(answer.0, StatusCode::OK, "{}", answer.2);
    }
    let (_, retry_after, json) = limited.expect("the key was never limited");
    assert!(retry_after.is_some());
    assert!(
        json["detail"].as_str().unwrap().contains("this key"),
        "{json}"
    );
}

#[tokio::test]
async fn a_busy_content_key_gets_a_429_with_retry_after() {
    let router = router();
    let peer = "203.0.113.10:5000";
    let mut limited = None;
    for _ in 0..2 * ApiLimits::CONTENT_PER_MINUTE {
        let answer = send(&router, POSTS, peer, Some(&content(CONTENT))).await;
        if answer.0 == StatusCode::TOO_MANY_REQUESTS {
            limited = Some(answer);
            break;
        }
    }
    let (_, retry_after, _) = limited.expect("the key was never limited");
    assert!(retry_after.is_some());
}
