//! A signed-in person lists, renames and removes their own passkeys over
//! the Admin API, with the session cookie and its CSRF token, and never
//! reaches someone else's.

use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::Request;
use chrono::{TimeZone, Utc};
use khalid_api::api;
use khalid_api::content_store::MemoryContentStore;
use khalid_api::keys::{MemoryKeyStore, Role};
use khalid_api::post_store::MemoryPostStore;
use khalid_auth::{
    AccountStore, Accounts, MemoryAccounts, MemoryPasskeys, MemorySessions, PasskeySite,
    PasskeyStore, Passkeys, PasswordHashing, SavedPasskey, SessionToken, staff_routes,
};
use khalid_core::{AbsoluteUrl, FixedClock};
use serde_json::{Value, json};
use tower::ServiceExt;

const BLOG: &str = "https://blog.example.com";

/// A site where Asif has the passkeys "Laptop" and "Phone" and Ben has
/// "Key", at `address`: the router, Asif's session, and the number of
/// Ben's passkey.
#[cfg(test)]
async fn site(address: &str) -> (Router, SessionToken, i64) {
    let at = Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap();
    let store = Arc::new(MemoryAccounts::new());
    let mut people = Vec::new();
    for (name, role) in [("asif", Role::Owner), ("ben", Role::Writer)] {
        let email = format!("{name}@example.com").parse().unwrap();
        people.push(store.create(&email, name, role, at).await.unwrap());
    }
    let kept = Arc::new(MemoryPasskeys::new());
    for (who, id, name) in [(0, 1, "Laptop"), (0, 2, "Phone"), (1, 3, "Key")] {
        let saved = SavedPasskey::from_store(vec![id; 16], "{}".to_owned());
        let account = people[who].id();
        assert!(kept.add(account, &saved, name, at).await.unwrap());
    }
    let ben_key = kept.listed(people[1].id()).await.unwrap()[0].id.as_row();
    let mut accounts = Accounts::new(
        store,
        Arc::new(MemorySessions::new()),
        PasswordHashing::fast_for_tests(),
        Arc::new(FixedClock::new(at)),
    );
    // A site at a bare IP address has no host name, so no passkeys.
    let address = AbsoluteUrl::parse(address).unwrap();
    if let Ok(passkey_site) = PasskeySite::for_address(&address) {
        accounts = accounts.with_passkeys(Passkeys::new(kept, passkey_site));
    }
    let token = accounts.begin(&people[0]).await.unwrap();
    let (router, _) = api::router_with_staff(
        Arc::new(MemoryContentStore::new(Vec::new())),
        Arc::new(MemoryPostStore::default()),
        Arc::new(MemoryKeyStore::new(Vec::new())),
        staff_routes(accounts.clone()),
        Arc::new(accounts),
    );
    (router, token, ben_key)
}

/// Sends `method` to `/session/passkeys/<path>` as the holder of
/// `session`, or as nobody.
#[cfg(test)]
async fn send(
    router: &Router,
    method: &str,
    path: &str,
    session: Option<&SessionToken>,
    body: Option<Value>,
) -> (u16, Value) {
    let uri = format!("/api/admin/v1/session/passkeys/{path}");
    let mut request = Request::builder().method(method).uri(uri);
    if let Some(token) = session {
        let cookie = token.set_cookie();
        let pair = cookie.expose().split(';').next().unwrap().to_owned();
        request = request
            .header("cookie", pair)
            .header("x-khalid-csrf", token.csrf().as_str());
    }
    let body = match body {
        Some(json) => {
            request = request.header("content-type", "application/json");
            Body::from(json.to_string())
        }
        None => Body::empty(),
    };
    let response = router.clone().oneshot(request.body(body).unwrap()).await;
    let response = response.unwrap();
    let status = response.status().as_u16();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[cfg(test)]
fn names(list: &Value) -> Vec<&str> {
    let list = list.as_array().unwrap();
    list.iter().map(|p| p["name"].as_str().unwrap()).collect()
}

#[tokio::test]
async fn a_person_lists_only_their_own_passkeys_and_never_their_keys() {
    let (router, asif, ben_key) = site(BLOG).await;
    let (status, list) = send(&router, "GET", "", Some(&asif), None).await;
    assert_eq!((status, names(&list)), (200, vec!["Laptop", "Phone"]));
    assert_eq!(list[0]["added_at"], "2026-10-27T09:00:00Z");
    assert_eq!(list[0]["last_used_at"], Value::Null);
    assert!(list[0].get("credential").is_none(), "never the key: {list}");
    assert_ne!(list[1]["id"], json!(ben_key));
    let (status, _) = send(&router, "GET", "", None, None).await;
    assert_eq!(status, 401, "the list needs the session");
}

#[tokio::test]
async fn a_site_without_passkeys_says_so() {
    let (router, asif, _) = site("http://127.0.0.1/").await;
    let (status, json) = send(&router, "GET", "", Some(&asif), None).await;
    assert_eq!(status, 404, "{json}");
    assert_eq!(json["title"], "Nothing is at this address");
}

#[tokio::test]
async fn a_person_renames_and_removes_their_own_passkey() {
    let (router, asif, _) = site(BLOG).await;
    let (_, list) = send(&router, "GET", "", Some(&asif), None).await;
    let phone = format!("{}/", list[1]["id"]);
    let rename = json!({ "name": "  Work phone " });
    let (status, json) = send(&router, "PATCH", &phone, Some(&asif), Some(rename)).await;
    assert_eq!(status, 204, "{json}");
    let (_, list) = send(&router, "GET", "", Some(&asif), None).await;
    let trimmed = names(&list);
    assert_eq!(trimmed, ["Laptop", "Work phone"], "the name is trimmed");
    let (status, json) = send(&router, "DELETE", &phone, Some(&asif), None).await;
    assert_eq!(status, 204, "{json}");
    let (_, list) = send(&router, "GET", "", Some(&asif), None).await;
    assert_eq!(names(&list), ["Laptop"]);
    let (status, _) = send(&router, "DELETE", &phone, Some(&asif), None).await;
    assert_eq!(status, 404, "already removed");
}

#[tokio::test]
async fn nobody_changes_another_persons_passkey_or_any_without_a_session() {
    let (router, asif, ben_key) = site(BLOG).await;
    let theirs = format!("{ben_key}/");
    let rename = json!({ "name": "Mine now" });
    for (method, path, body) in [
        ("PATCH", theirs.as_str(), Some(rename.clone())),
        ("DELETE", theirs.as_str(), None),
        ("DELETE", "laptop/", None),
    ] {
        let (status, json) = send(&router, method, path, Some(&asif), body).await;
        assert_eq!(status, 404, "{method} {path}: {json}");
        assert_eq!(json["type"], "https://khalid.dev/problems/not-found");
    }
    for (method, body) in [("PATCH", Some(rename)), ("DELETE", None)] {
        let (status, _) = send(&router, method, &theirs, None, body).await;
        assert_eq!(status, 401, "{method} needs the session");
    }
}

#[tokio::test]
async fn a_blank_or_long_name_is_refused() {
    let (router, asif, _) = site(BLOG).await;
    let (_, list) = send(&router, "GET", "", Some(&asif), None).await;
    let laptop = format!("{}/", list[0]["id"]);
    for name in ["   ".to_owned(), "x".repeat(61)] {
        let body = Some(json!({ "name": name }));
        let (status, json) = send(&router, "PATCH", &laptop, Some(&asif), body).await;
        assert_eq!(status, 422, "{name:?}: {json}");
        assert!(json["detail"].as_str().unwrap().contains("60"), "{json}");
    }
    let (_, list) = send(&router, "GET", "", Some(&asif), None).await;
    assert_eq!(names(&list), ["Laptop", "Phone"], "nothing changed");
}
