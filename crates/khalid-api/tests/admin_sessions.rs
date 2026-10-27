//! A staff session cookie opens the Admin API only with its CSRF token,
//! staff tokens keep working beside it, and staff routes from another
//! crate join the router and the document.

mod common;

use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use axum::{Extension, Json};
use chrono::{TimeZone, Utc};
use khalid_api::api;
use khalid_api::content_store::BoxFuture;
use khalid_api::keys::{
    Access, KeyHashing, KeyId, KeyStoreError, KeyToken, MemoryKeyStore, Principal, Role, StoredKey,
};
use khalid_api::post_store::MemoryPostStore;
use khalid_api::resources::AdminPost;
use khalid_api::session_check::{SessionCheck, StaffRoutes};
use khalid_core::Slug;
use serde_json::Value;
use tower::ServiceExt;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

/// A staff token made up for these tests. It opens nothing real.
const EDITOR: &str =
    "ks_5555666677778888_0000111122223333444455556666777788889999aaaabbbbccccddddeeeeffff";
const COOKIE: &str = "theme=light; __Host-khalid-session=good";
const CSRF: &str = "right";

/// Sessions where the cookie `good` with the token `right` is an editor.
struct Sessions;

impl SessionCheck for Sessions {
    fn check<'a>(
        &'a self,
        cookies: &'a str,
        csrf: Option<&'a str>,
    ) -> BoxFuture<'a, Result<Option<Principal>, KeyStoreError>> {
        let valid = cookies.contains("__Host-khalid-session=good") && csrf == Some(CSRF);
        let principal = valid.then(|| {
            let id = KeyId::try_new("00000000000000aa").ok()?;
            Some(Principal::new(id, Access::Staff(Role::Editor)))
        });
        Box::pin(async move { Ok(principal.flatten()) })
    }
}

/// Says who signed in.
#[utoipa::path(get, path = "/whoami/", tag = "test", responses((status = 200, description = "Who")))]
async fn whoami(Extension(principal): Extension<Principal>) -> Json<String> {
    Json(principal.id().as_str().to_owned())
}

/// Needs nobody.
#[utoipa::path(get, path = "/hello/", tag = "test", responses((status = 200, description = "Hello")))]
async fn hello() -> Json<&'static str> {
    Json("hello")
}

#[cfg(test)]
fn router() -> (axum::Router, utoipa::openapi::OpenApi) {
    let hashing = KeyHashing::fast_for_tests();
    let token: KeyToken = EDITOR.parse().unwrap();
    let keys = MemoryKeyStore::new(vec![
        StoredKey::for_staff(&token, Role::Editor, &hashing).unwrap(),
    ]);
    let at = Utc.with_ymd_and_hms(2026, 10, 20, 8, 30, 0).unwrap();
    let slug = Slug::try_new("choosing-anchor-text").unwrap();
    let draft = AdminPost::new("draft", slug, "Choosing anchor text", "Anchor text.", at);
    let staff = StaffRoutes {
        open: OpenApiRouter::new().routes(routes!(hello)),
        signed_in: OpenApiRouter::new().routes(routes!(whoami)),
    };
    api::router_with_staff(
        Arc::new(common::store()),
        Arc::new(MemoryPostStore::new(vec![draft])),
        Arc::new(keys),
        staff,
        Arc::new(Sessions),
    )
}

#[cfg(test)]
async fn get(uri: &str, headers: &[(&str, &str)]) -> (StatusCode, Value) {
    let mut request = Request::get(uri);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let response = router()
        .0
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (status, serde_json::from_slice(&body).unwrap())
}

#[tokio::test]
async fn a_session_cookie_with_its_csrf_token_reads_a_draft() {
    let headers = [("cookie", COOKIE), ("x-khalid-csrf", CSRF)];
    let (status, json) = get("/api/admin/v1/posts/draft/", &headers).await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["posts"][0]["status"], "draft");
}

#[tokio::test]
async fn a_cookie_without_its_csrf_token_is_refused() {
    let no_token = [("cookie", COOKIE)];
    let wrong_token = [("cookie", COOKIE), ("x-khalid-csrf", "wrong")];
    let stolen_token = [("x-khalid-csrf", CSRF)];
    for headers in [&no_token[..], &wrong_token[..], &stolen_token[..]] {
        let (status, json) = get("/api/admin/v1/posts/draft/", headers).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{headers:?}");
        assert_eq!(json["type"], "https://khalid.dev/problems/unauthorized");
    }
}

#[tokio::test]
async fn a_staff_token_still_works_and_wins_over_a_cookie() {
    let token = format!("Khalid {EDITOR}");
    let (status, _) = get("/api/admin/v1/posts/draft/", &[("authorization", &token)]).await;
    assert_eq!(status, StatusCode::OK);
    let wrong = [
        ("authorization", "Khalid nonsense"),
        ("cookie", COOKIE),
        ("x-khalid-csrf", CSRF),
    ];
    let (status, _) = get("/api/admin/v1/posts/draft/", &wrong).await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "a bad key is not rescued by a cookie"
    );
}

#[tokio::test]
async fn staff_routes_join_the_admin_api_and_its_document() {
    let headers = [("cookie", COOKIE), ("x-khalid-csrf", CSRF)];
    assert_eq!(
        get("/api/admin/v1/whoami/", &headers).await,
        (StatusCode::OK, Value::from("00000000000000aa"))
    );
    assert_eq!(
        get("/api/admin/v1/whoami/", &[]).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        get("/api/admin/v1/hello/", &[]).await,
        (StatusCode::OK, Value::from("hello"))
    );
    let (_, document) = router();
    let paths: Vec<&String> = document.paths.paths.keys().collect();
    assert!(
        paths.iter().any(|path| *path == "/api/admin/v1/whoami/"),
        "{paths:?}"
    );
    assert!(
        paths.iter().any(|path| *path == "/api/admin/v1/hello/"),
        "{paths:?}"
    );
    let schemes = document
        .components
        .as_ref()
        .unwrap()
        .security_schemes
        .keys();
    assert!(schemes.into_iter().any(|name| name == "staff_session"));
}
