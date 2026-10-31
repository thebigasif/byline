//! The super-admin panel's routes over HTTP: only a person signed in on
//! the home site, whom the process lists as a super admin, sees and adds
//! sites. Keys, other staff, stale sessions and missing CSRF tokens are
//! refused, and a refused request adds nothing.

use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use chrono::{TimeZone, Utc};
use khalid_api::api;
use khalid_api::content_store::MemoryContentStore;
use khalid_api::keys::{KeyHashing, KeyToken, MemoryKeyStore, Role, StoredKey};
use khalid_api::post_store::MemoryPostStore;
use khalid_auth::{
    AccountStore, Accounts, MemoryAccounts, MemorySessions, MemorySites, Password, PasswordHashing,
    site_domain, staff_routes, with_sites,
};
use khalid_core::FixedClock;
use serde_json::{Value, json};
use tower::ServiceExt;

const PASSWORD: &str = "correct horse battery";
/// An owner's staff token made up for these tests. It opens nothing real.
const OWNER_TOKEN: &str =
    "ks_5555666677778888_0000111122223333444455556666777788889999aaaabbbbccccddddeeeeffff";
const SITES: &str = "/api/admin/v1/sites/";

/// The home site `example.com` with an owner, who is a super admin, and
/// an admin, who is not; both sign in with the same password.
#[cfg(test)]
async fn router() -> Router {
    let at = Utc.with_ymd_and_hms(2026, 10, 31, 9, 0, 0).unwrap();
    let store = Arc::new(MemoryAccounts::new());
    let hashing = PasswordHashing::fast_for_tests();
    let hash = hashing
        .hash(Password::new(PASSWORD.to_owned()).unwrap())
        .await
        .unwrap();
    for role in [Role::Owner, Role::Admin] {
        let email = format!("{}@example.com", role.as_str()).parse().unwrap();
        let account = store.create(&email, role.as_str(), role, at).await.unwrap();
        store.set_password(account.id(), &hash).await.unwrap();
    }
    let accounts = Accounts::new(
        store,
        Arc::new(MemorySessions::new()),
        hashing,
        Arc::new(FixedClock::new(at)),
    );
    let home = site_domain("example.com").unwrap();
    let sites = MemorySites::new(home, vec!["owner@example.com".parse().unwrap()]);
    let token: KeyToken = OWNER_TOKEN.parse().unwrap();
    let key = StoredKey::for_staff(&token, Role::Owner, &KeyHashing::fast_for_tests()).unwrap();
    let staff = with_sites(
        staff_routes(accounts.clone()),
        accounts.clone(),
        Arc::new(sites),
    );
    let (router, _) = api::router_with_staff(
        Arc::new(MemoryContentStore::new(Vec::new())),
        Arc::new(MemoryPostStore::default()),
        Arc::new(MemoryKeyStore::new(vec![key])),
        staff,
        Arc::new(accounts),
    );
    router
}

#[cfg(test)]
async fn send(router: &Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

/// Signs `role`'s account in; returns the cookie pair and CSRF token.
#[cfg(test)]
async fn sign_in(router: &Router, role: Role) -> (String, String) {
    let body = json!({ "email": format!("{}@example.com", role.as_str()), "password": PASSWORD });
    let request = Request::post("/api/admin/v1/session/")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let response = router.clone().oneshot(request).await.unwrap();
    let cookie = response.headers()["set-cookie"].to_str().unwrap();
    let pair = cookie.split(';').next().unwrap().to_owned();
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    (pair, json["csrf"].as_str().unwrap().to_owned())
}

/// A request with `headers`, and `body` as JSON when there is one.
#[cfg(test)]
fn request(method: &str, headers: &[(&str, &str)], body: Option<&str>) -> Request<Body> {
    let mut request = Request::builder()
        .method(method)
        .uri(SITES)
        .header("content-type", "application/json");
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let body = body.map_or_else(Body::empty, |body| Body::from(body.to_owned()));
    request.body(body).unwrap()
}

#[cfg(test)]
fn domains(json: &Value) -> Vec<(String, bool)> {
    json["sites"]
        .as_array()
        .unwrap()
        .iter()
        .map(|site| {
            let domain = site["domain"].as_str().unwrap().to_owned();
            (domain, site["home"].as_bool().unwrap())
        })
        .collect()
}

#[tokio::test]
async fn a_super_admin_lists_the_sites_and_adds_one_by_its_domain() {
    let router = router().await;
    let (cookie, csrf) = sign_in(&router, Role::Owner).await;
    let signed_in = [
        ("cookie", cookie.as_str()),
        ("x-khalid-csrf", csrf.as_str()),
    ];

    let (status, json) = send(&router, request("GET", &signed_in, None)).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(domains(&json), [("example.com".to_owned(), true)]);

    let news = r#"{"domain":"News.Example.com"}"#;
    let (status, json) = send(&router, request("POST", &signed_in, Some(news))).await;
    assert_eq!(status, StatusCode::CREATED, "{json}");
    assert_eq!(json, json!({ "domain": "news.example.com", "home": false }));

    let (status, json) = send(&router, request("POST", &signed_in, Some(news))).await;
    assert_eq!(status, StatusCode::CONFLICT, "{json}");
    assert_eq!(json["type"], "https://khalid.dev/problems/conflict");

    for refused in [
        r#"{"domain":"../etc"}"#,
        r#"{"domain":"a b.com"}"#,
        "{",
        r#"{"host":"x.com"}"#,
    ] {
        let (status, json) = send(&router, request("POST", &signed_in, Some(refused))).await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{refused}: {json}"
        );
    }

    let (_, json) = send(&router, request("GET", &signed_in, None)).await;
    let want = [
        ("example.com".to_owned(), true),
        ("news.example.com".to_owned(), false),
    ];
    assert_eq!(domains(&json), want);
}

/// Who calls, with which headers, and the status they must get.
type Case<'a> = (&'a str, Vec<(&'a str, &'a str)>, StatusCode);

#[tokio::test]
async fn everyone_but_a_signed_in_super_admin_is_refused_and_adds_nothing() {
    let router = router().await;
    let (owner_cookie, owner_csrf) = sign_in(&router, Role::Owner).await;
    let (admin_cookie, admin_csrf) = sign_in(&router, Role::Admin).await;
    let key = format!("Khalid {OWNER_TOKEN}");
    let body = Some(r#"{"domain":"news.example.com"}"#);
    let cases: [Case<'_>; 5] = [
        ("nobody", vec![], StatusCode::UNAUTHORIZED),
        (
            "a staff member the server does not list",
            vec![("cookie", &admin_cookie), ("x-khalid-csrf", &admin_csrf)],
            StatusCode::FORBIDDEN,
        ),
        (
            "the owner's staff token, a key",
            vec![("authorization", &key)],
            StatusCode::FORBIDDEN,
        ),
        (
            "the super admin's cookie without its CSRF token",
            vec![("cookie", &owner_cookie)],
            StatusCode::UNAUTHORIZED,
        ),
        (
            "the super admin's cookie with a wrong CSRF token, beside a key",
            vec![
                ("authorization", &key),
                ("cookie", &owner_cookie),
                ("x-khalid-csrf", &admin_csrf),
            ],
            StatusCode::UNAUTHORIZED,
        ),
    ];
    for (who, headers, want) in &cases {
        for method in ["GET", "POST"] {
            let body = body.filter(|_| method == "POST");
            let (status, json) = send(&router, request(method, headers, body)).await;
            assert_eq!(status, *want, "{who}, {method}: {json}");
            assert!(json["detail"].is_string(), "{who}, {method}: {json}");
        }
    }

    let owner = [
        ("cookie", owner_cookie.as_str()),
        ("x-khalid-csrf", owner_csrf.as_str()),
    ];
    let (_, json) = send(&router, request("GET", &owner, None)).await;
    assert_eq!(domains(&json), [("example.com".to_owned(), true)]);
}

#[tokio::test]
async fn a_super_admin_who_signed_out_is_refused() {
    let router = router().await;
    let (cookie, csrf) = sign_in(&router, Role::Owner).await;
    let signed_in = [
        ("cookie", cookie.as_str()),
        ("x-khalid-csrf", csrf.as_str()),
    ];
    let mut sign_out = request("DELETE", &signed_in, None);
    *sign_out.uri_mut() = "/api/admin/v1/session/".parse().unwrap();
    let (status, _) = send(&router, sign_out).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, json) = send(&router, request("GET", &signed_in, None)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{json}");
}
