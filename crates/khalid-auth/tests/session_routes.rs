//! Signing in over HTTP, end to end through the Admin API router, with
//! the session cookie and its CSRF token.

use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{HeaderMap, Request, StatusCode};
use chrono::{TimeZone, Utc};
use khalid_api::api;
use khalid_api::content_store::MemoryContentStore;
use khalid_api::keys::{MemoryKeyStore, Role};
use khalid_api::post_store::MemoryPostStore;
use khalid_auth::{
    AccountStore, Accounts, MemoryAccounts, MemorySessions, Password, PasswordHashing, staff_routes,
};
use khalid_core::FixedClock;
use serde_json::{Value, json};
use tower::ServiceExt;

/// A site whose owner, Asif, has the password "correct horse battery".
#[cfg(test)]
async fn router() -> Router {
    let at = Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap();
    let store = Arc::new(MemoryAccounts::new());
    let hashing = PasswordHashing::fast_for_tests();
    let email = "asif@example.com".parse().unwrap();
    let owner = store.create(&email, "Asif", Role::Owner, at).await.unwrap();
    let password = Password::new("correct horse battery".to_owned()).unwrap();
    let hash = hashing.hash(password).await.unwrap();
    store.set_password(owner.id(), &hash).await.unwrap();
    let accounts = Accounts::new(
        store,
        Arc::new(MemorySessions::new()),
        hashing,
        Arc::new(FixedClock::new(at)),
    );
    let (router, _) = api::router_with_staff(
        Arc::new(MemoryContentStore::new(Vec::new())),
        Arc::new(MemoryPostStore::default()),
        Arc::new(MemoryKeyStore::new(Vec::new())),
        staff_routes(accounts.clone()),
        Arc::new(accounts),
    );
    router
}

#[cfg(test)]
async fn send(router: &Router, request: Request<Body>) -> (StatusCode, HeaderMap, Value) {
    let response = router.clone().oneshot(request).await.unwrap();
    let (parts, body) = response.into_parts();
    let body = to_bytes(body, 1024 * 1024).await.unwrap();
    let json = serde_json::from_slice(&body).unwrap_or(Value::Null);
    (parts.status, parts.headers, json)
}

#[cfg(test)]
fn sign_in_request(email: &str, password: &str) -> Request<Body> {
    let body = json!({ "email": email, "password": password }).to_string();
    Request::post("/api/admin/v1/session/")
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap()
}

#[cfg(test)]
async fn signed_in(router: &Router) -> (String, String) {
    let (status, headers, json) = send(
        router,
        sign_in_request("Asif@example.com", "correct horse battery"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{json}");
    let set_cookie = headers["set-cookie"].to_str().unwrap();
    assert!(
        set_cookie.contains("HttpOnly") && set_cookie.contains("Secure"),
        "{set_cookie}"
    );
    let pair = set_cookie.split(';').next().unwrap().to_owned();
    assert_eq!(json["account"]["role"], "owner");
    (pair, json["csrf"].as_str().unwrap().to_owned())
}

#[cfg(test)]
fn with_session(method: &str, uri: &str, cookie: &str, csrf: Option<&str>) -> Request<Body> {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header("cookie", cookie);
    if let Some(csrf) = csrf {
        request = request.header("x-khalid-csrf", csrf);
    }
    request.body(Body::empty()).unwrap()
}

#[tokio::test]
async fn signing_in_sets_a_cookie_that_finds_the_account_with_its_csrf_token() {
    let router = router().await;
    let (cookie, csrf) = signed_in(&router).await;
    let request = with_session("GET", "/api/admin/v1/session/", &cookie, Some(&csrf));
    let (status, _, json) = send(&router, request).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["account"]["email"], "asif@example.com");
    assert_eq!(json["csrf"], csrf.as_str());
}

#[tokio::test]
async fn a_wrong_password_an_unknown_email_and_a_short_one_get_one_401() {
    let router = router().await;
    for (email, password) in [
        ("asif@example.com", "incorrect horse"),
        ("nobody@example.com", "correct horse battery"),
        ("asif@example.com", "short"),
        ("not an email", "correct horse battery"),
    ] {
        let (status, headers, json) = send(&router, sign_in_request(email, password)).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{email} {password}");
        assert!(!headers.contains_key("set-cookie"));
        assert_eq!(json["type"], "https://khalid.dev/problems/unauthorized");
    }
}

#[tokio::test]
async fn the_sixth_try_after_five_failures_is_a_429_with_retry_after() {
    let router = router().await;
    for _ in 0..5 {
        send(
            &router,
            sign_in_request("asif@example.com", "incorrect horse"),
        )
        .await;
    }
    let (status, headers, json) = send(
        &router,
        sign_in_request("asif@example.com", "correct horse battery"),
    )
    .await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS, "{json}");
    assert_eq!(headers["retry-after"], "960");
    assert!(
        json["fix"].as_str().unwrap().contains("16 minutes"),
        "{json}"
    );
}

#[tokio::test]
async fn the_session_routes_need_the_csrf_token_too() {
    let router = router().await;
    let (cookie, _) = signed_in(&router).await;
    let request = with_session("GET", "/api/admin/v1/session/", &cookie, None);
    assert_eq!(send(&router, request).await.0, StatusCode::UNAUTHORIZED);
    let request = with_session("GET", "/api/admin/v1/session/", &cookie, Some("forged"));
    assert_eq!(send(&router, request).await.0, StatusCode::UNAUTHORIZED);
}
