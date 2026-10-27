//! Signing in by email link over HTTP, end to end through the Admin API
//! router: ask, receive, redeem once.

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
    AccountStore, Accounts, Links, MemoryAccounts, MemoryLinks, MemoryMailbox, MemorySessions,
    PasswordHashing, staff_routes,
};
use khalid_core::FixedClock;
use serde_json::{Value, json};
use tower::ServiceExt;

/// A site where Mia, a writer with no password, can get links, its
/// mailbox, and a site that cannot send mail.
#[cfg(test)]
async fn routers() -> (Router, Arc<MemoryMailbox>, Router) {
    let at = Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap();
    let mailbox = Arc::new(MemoryMailbox::new());
    let make = |links: Option<Links>| {
        let store = Arc::new(MemoryAccounts::new());
        let accounts = Accounts::new(
            Arc::<MemoryAccounts>::clone(&store),
            Arc::new(MemorySessions::new()),
            PasswordHashing::fast_for_tests(),
            Arc::new(FixedClock::new(at)),
        );
        let accounts = match links {
            Some(links) => accounts.with_links(links),
            None => accounts,
        };
        let (router, _) = api::router_with_staff(
            Arc::new(MemoryContentStore::new(Vec::new())),
            Arc::new(MemoryPostStore::default()),
            Arc::new(MemoryKeyStore::new(Vec::new())),
            staff_routes(accounts.clone()),
            Arc::new(accounts),
        );
        (router, store)
    };
    let links = Links {
        store: Arc::new(MemoryLinks::new()),
        sender: Arc::<MemoryMailbox>::clone(&mailbox),
        admin: "https://blog.example.com/admin".to_owned(),
    };
    let (router, store) = make(Some(links));
    let email = "mia@example.com".parse().unwrap();
    store.create(&email, "Mia", Role::Writer, at).await.unwrap();
    (router, mailbox, make(None).0)
}

#[cfg(test)]
async fn post(router: &Router, uri: &str, body: &Value) -> (StatusCode, HeaderMap, Value) {
    let request = Request::post(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let response = router.clone().oneshot(request).await.unwrap();
    let (parts, body) = response.into_parts();
    let body = to_bytes(body, 1024 * 1024).await.unwrap();
    (
        parts.status,
        parts.headers,
        serde_json::from_slice(&body).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn a_link_asked_for_by_email_signs_in_once() {
    let (router, mailbox, _) = routers().await;
    let (status, _, _) = post(
        &router,
        "/api/admin/v1/session/link/",
        &json!({ "email": "nobody@example.com" }),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::ACCEPTED,
        "the same answer for an unknown email"
    );
    let (status, _, _) = post(
        &router,
        "/api/admin/v1/session/link/",
        &json!({ "email": "mia@example.com" }),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let sent = mailbox.sent();
    assert_eq!(sent.len(), 1, "only the real account got mail");
    let (_, link) = sent.first().unwrap();
    let token = link.split_once("#token=").unwrap().1.to_owned();
    let redeem = json!({ "token": token });
    let (status, headers, json) =
        post(&router, "/api/admin/v1/session/link/redeem/", &redeem).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert!(
        headers["set-cookie"]
            .to_str()
            .unwrap()
            .starts_with("__Host-khalid-session=")
    );
    assert_eq!(json["account"]["email"], "mia@example.com");
    let (status, headers, _) = post(&router, "/api/admin/v1/session/link/redeem/", &redeem).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "a link works once");
    assert!(!headers.contains_key("set-cookie"));
}

#[tokio::test]
async fn a_bad_request_or_a_made_up_token_is_refused() {
    let (router, _, _) = routers().await;
    let (status, _, _) = post(
        &router,
        "/api/admin/v1/session/link/",
        &json!({ "email": "not an email" }),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let made_up = json!({ "token": "0".repeat(64) });
    let (status, _, _) = post(&router, "/api/admin/v1/session/link/redeem/", &made_up).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn a_site_without_mail_says_links_are_off() {
    let (_, _, off) = routers().await;
    let (status, _, json) = post(
        &off,
        "/api/admin/v1/session/link/",
        &json!({ "email": "mia@example.com" }),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(json["fix"].as_str().unwrap().contains("password"), "{json}");
}
