//! Both APIs behind one router, and the one `OpenAPI` document.

use std::sync::Arc;

use axum::routing::get;
use axum::{Json, Router};
use utoipa::openapi::OpenApi as Document;
use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};
use utoipa::{Modify, OpenApi};
use utoipa_axum::router::OpenApiRouter;

use crate::content_store::ContentStore;
use crate::keys::KeyStore;
use crate::post_store::PostStore;
use crate::problem::Problem;
use crate::publish_door::{PublishDoor, Silent};
use crate::redirect_desk::RedirectDesk;
use crate::{admin_api, admin_redirects, api_version, content_api};

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
        ];
        for (name, scheme) in schemes {
            components.add_security_scheme(name, SecurityScheme::ApiKey(scheme));
        }
    }
}

/// The Content API under `/api/content/v1/`, the Admin API under
/// `/api/admin/v1/`, and `/api/openapi.json`, with the document every
/// route declares (RULES.md BL-API-4). The document itself needs no key.
/// A post that goes live is announced to nobody; see [`router_with_door`].
pub fn router(
    content: Arc<dyn ContentStore>,
    posts: Arc<dyn PostStore>,
    keys: Arc<dyn KeyStore>,
) -> (Router, Document) {
    let door = PublishDoor::new(posts, Arc::new(Silent));
    router_with_door(content, Arc::new(door), keys)
}

/// The same router, publishing through `door`, which `khalid serve` shares
/// with its scheduler so every post that goes live is announced.
/// Its redirect routes keep nothing; see [`router_with_redirects`].
pub fn router_with_door(
    content: Arc<dyn ContentStore>,
    door: Arc<PublishDoor>,
    keys: Arc<dyn KeyStore>,
) -> (Router, Document) {
    router_with_redirects(content, door, keys, RedirectDesk::none())
}

/// The same router, with the redirect routes under
/// `/api/admin/v1/redirects/` reading and writing through `redirects`.
pub fn router_with_redirects(
    content: Arc<dyn ContentStore>,
    door: Arc<PublishDoor>,
    keys: Arc<dyn KeyStore>,
    redirects: RedirectDesk,
) -> (Router, Document) {
    let admin = admin_api::routes(door, Arc::clone(&keys))
        .merge(admin_redirects::routes(redirects, Arc::clone(&keys)));
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
