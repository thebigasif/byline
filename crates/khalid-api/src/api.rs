//! Both APIs behind one router, and the one `OpenAPI` document.

use std::sync::Arc;

use axum::middleware;
use axum::routing::get;
use axum::{Json, Router};
use utoipa::openapi::OpenApi as Document;
use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};
use utoipa::{Modify, OpenApi};
use utoipa_axum::router::OpenApiRouter;

use crate::admin_key::{AdminDoor, require_admin_key};
use crate::api_version::negotiate_version;
use crate::content_store::ContentStore;
use crate::keys::KeyStore;
use crate::post_store::PostStore;
use crate::problem::Problem;
use crate::session_check::{SessionCheck, StaffRoutes};
use crate::{admin_api, api_version, content_api};

/// The document's title, the schemas every route shares and the ways a
/// key may be sent.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Khalid API",
        description = "The Content API and the Admin API of a Khalid blog."
    ),
    components(schemas(Problem)),
    modifiers(&KeySchemes)
)]
pub(crate) struct ApiDoc;

/// Adds the ways to send a key to the document.
struct KeySchemes;

impl Modify for KeySchemes {
    fn modify(&self, document: &mut Document) {
        let components = document.components.get_or_insert_with(Default::default);
        let schemes = [
            (
                "content_key",
                ApiKey::Header(ApiKeyValue::with_description(
                    "Authorization",
                    "The site's content key, as Khalid-Content followed by the key.",
                )),
            ),
            (
                "content_key_query",
                ApiKey::Query(ApiKeyValue::with_description(
                    "key",
                    "The site's content key.",
                )),
            ),
            (
                "admin_key",
                ApiKey::Header(ApiKeyValue::with_description(
                    "Authorization",
                    "An integration key or staff token, as Khalid followed by the key.",
                )),
            ),
            (
                "staff_session",
                ApiKey::Cookie(ApiKeyValue::with_description(
                    "__Host-khalid-session",
                    "A signed-in staff session. Every request with it also sends the session's \
                     CSRF token in the x-khalid-csrf header.",
                )),
            ),
        ];
        for (name, scheme) in schemes {
            components.add_security_scheme(name, SecurityScheme::ApiKey(scheme));
        }
    }
}

/// The Content API under `/api/content/v1/`, the Admin API under
/// `/api/admin/v1/`, and `/api/openapi.json`, with the document every
/// route declares (RULES.md BL-API-4). The document itself needs no key.
pub fn router(
    content: Arc<dyn ContentStore>,
    posts: Arc<dyn PostStore>,
    keys: Arc<dyn KeyStore>,
) -> (Router, Document) {
    let door = AdminDoor {
        keys: Arc::clone(&keys),
        sessions: None,
    };
    let api = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest(content_api::BASE, content_api::routes(content, keys))
        .nest(admin_api::BASE, admin_api::routes(posts, door));
    finish(api)
}

/// [`router`] with staff accounts: `staff` routes join the Admin API, and
/// `sessions` lets a staff session cookie in wherever a key is needed.
/// Each of `staff.open` and `staff.signed_in` must hold at least one route.
pub fn router_with_staff(
    content: Arc<dyn ContentStore>,
    posts: Arc<dyn PostStore>,
    keys: Arc<dyn KeyStore>,
    staff: StaffRoutes,
    sessions: Arc<dyn SessionCheck>,
) -> (Router, Document) {
    let door = AdminDoor {
        keys: Arc::clone(&keys),
        sessions: Some(sessions),
    };
    let signed_in = staff.signed_in.route_layer(middleware::from_fn_with_state(
        door.clone(),
        require_admin_key,
    ));
    let staff = staff
        .open
        .merge(signed_in)
        .route_layer(middleware::from_fn(negotiate_version));
    let admin = admin_api::routes(posts, door).merge(staff);
    let api = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest(content_api::BASE, content_api::routes(content, keys))
        .nest(admin_api::BASE, admin);
    finish(api)
}

/// Splits `api` into the router and its document, lists
/// `Accept-Version` on every operation, and serves the document.
pub(crate) fn finish(api: OpenApiRouter) -> (Router, Document) {
    let (router, mut document) = api.split_for_parts();
    api_version::document(&mut document);
    let served = Json(document.clone());
    let router = router.route("/api/openapi.json", get(move || async move { served }));
    (router, document)
}
