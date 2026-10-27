//! Every role boundary over HTTP: each role, signed in, invites each role,
//! and only what the permission table allows goes through.

use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use chrono::{TimeZone, Utc};
use khalid_api::api;
use khalid_api::content_store::MemoryContentStore;
use khalid_api::keys::{MemoryKeyStore, Role};
use khalid_api::post_store::MemoryPostStore;
use khalid_auth::{
    AccountStore, Accounts, MemoryAccounts, MemorySessions, Password, PasswordHashing, may_assign,
    may_manage, staff_routes,
};
use khalid_core::FixedClock;
use serde_json::{Value, json};
use tower::ServiceExt;

const PASSWORD: &str = "correct horse battery";

/// A site with one account per role, each with the same password.
#[cfg(test)]
async fn router() -> Router {
    let at = Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap();
    let store = Arc::new(MemoryAccounts::new());
    let hashing = PasswordHashing::fast_for_tests();
    let hash = hashing
        .hash(Password::new(PASSWORD.to_owned()).unwrap())
        .await
        .unwrap();
    for role in Role::ALL {
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

#[cfg(test)]
fn as_staff(
    method: &str,
    uri: &str,
    session: &(String, String),
    body: Option<Value>,
) -> Request<Body> {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("cookie", &session.0)
        .header("x-khalid-csrf", &session.1)
        .header("content-type", "application/json");
    request
        .body(body.map_or_else(Body::empty, |body| Body::from(body.to_string())))
        .unwrap()
}

#[tokio::test]
async fn each_role_invites_exactly_the_roles_it_may_manage() {
    let router = router().await;
    for actor in Role::ALL {
        let session = sign_in(&router, actor).await;
        for target in Role::ALL {
            let email = format!("{}-made-{}@example.com", actor.as_str(), target.as_str());
            let body = json!({ "email": email, "name": "New Person", "role": target.as_str() });
            let (status, json) = send(
                &router,
                as_staff("POST", "/api/admin/v1/staff/", &session, Some(body)),
            )
            .await;
            let want = if may_manage(actor, target) {
                StatusCode::CREATED
            } else {
                StatusCode::FORBIDDEN
            };
            assert_eq!(status, want, "{actor:?} inviting {target:?}: {json}");
        }
    }
}

#[tokio::test]
async fn only_roles_that_manage_staff_see_the_list() {
    let router = router().await;
    for actor in Role::ALL {
        let session = sign_in(&router, actor).await;
        let (status, json) = send(
            &router,
            as_staff("GET", "/api/admin/v1/staff/", &session, None),
        )
        .await;
        let manages = matches!(actor, Role::Owner | Role::Admin | Role::Editor);
        let want = if manages {
            StatusCode::OK
        } else {
            StatusCode::FORBIDDEN
        };
        assert_eq!(status, want, "{actor:?}: {json}");
        if manages {
            assert_eq!(json["staff"].as_array().unwrap().len(), 5);
        }
    }
}

#[tokio::test]
async fn an_email_already_in_use_is_a_409_and_a_bad_one_a_422() {
    let router = router().await;
    let session = sign_in(&router, Role::Owner).await;
    let again = json!({ "email": "Writer@example.com", "name": "Again", "role": "writer" });
    let (status, _) = send(
        &router,
        as_staff("POST", "/api/admin/v1/staff/", &session, Some(again)),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let bad = json!({ "email": "not an email", "name": "Bad", "role": "writer" });
    let (status, _) = send(
        &router,
        as_staff("POST", "/api/admin/v1/staff/", &session, Some(bad)),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

/// The account number of `role`'s account: they were made in the order
/// `Role::ALL` lists them, from 1.
#[cfg(test)]
fn number(role: Role) -> usize {
    Role::ALL.iter().position(|r| *r == role).unwrap() + 1
}

#[tokio::test]
async fn each_role_moves_exactly_the_accounts_it_may_between_roles_it_may_give() {
    for actor in Role::ALL {
        for target in Role::ALL.into_iter().filter(|target| *target != actor) {
            for to in [Role::Admin, Role::Contributor] {
                let router = router().await;
                let session = sign_in(&router, actor).await;
                let uri = format!("/api/admin/v1/staff/{}/", number(target));
                let body = json!({ "role": to.as_str() });
                let (status, json) =
                    send(&router, as_staff("PUT", &uri, &session, Some(body))).await;
                let want = if may_assign(actor, target, to) {
                    StatusCode::OK
                } else {
                    StatusCode::FORBIDDEN
                };
                assert_eq!(
                    status, want,
                    "{actor:?} moving {target:?} to {to:?}: {json}"
                );
            }
        }
    }
}

#[tokio::test]
async fn each_role_removes_exactly_the_accounts_it_may_manage() {
    for actor in Role::ALL {
        for target in Role::ALL.into_iter().filter(|target| *target != actor) {
            let router = router().await;
            let session = sign_in(&router, actor).await;
            let uri = format!("/api/admin/v1/staff/{}/", number(target));
            let (status, _) = send(&router, as_staff("DELETE", &uri, &session, None)).await;
            let want = if may_manage(actor, target) {
                StatusCode::NO_CONTENT
            } else {
                StatusCode::FORBIDDEN
            };
            assert_eq!(status, want, "{actor:?} removing {target:?}");
        }
    }
}

#[tokio::test]
async fn only_the_signed_in_owner_hands_the_site_over() {
    let router = router().await;
    let uri = format!("/api/admin/v1/staff/{}/owner/", number(Role::Editor));
    let admin = sign_in(&router, Role::Admin).await;
    assert_eq!(
        send(&router, as_staff("POST", &uri, &admin, None)).await.0,
        StatusCode::FORBIDDEN
    );
    let owner = sign_in(&router, Role::Owner).await;
    let (status, json) = send(&router, as_staff("POST", &uri, &owner, None)).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["staff"][0]["role"], "owner");
    let (status, json) = send(
        &router,
        as_staff("GET", "/api/admin/v1/staff/", &owner, None),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "the old owner is an admin now and still manages staff"
    );
    let roles: Vec<&str> = json["staff"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["role"].as_str().unwrap())
        .collect();
    assert_eq!(
        roles.iter().filter(|role| **role == "owner").count(),
        1,
        "{roles:?}"
    );
    let unknown = as_staff(
        "PUT",
        "/api/admin/v1/staff/99/",
        &owner,
        Some(json!({ "role": "writer" })),
    );
    assert_eq!(send(&router, unknown).await.0, StatusCode::NOT_FOUND);
}
