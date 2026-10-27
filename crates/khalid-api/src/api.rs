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
use crate::comment_moderation::{CommentModeration, NoComments};
use crate::content_store::ContentStore;
use crate::keys::KeyStore;
use crate::media_api::{self, MediaLibrary};
use crate::post_store::PostStore;
use crate::problem::Problem;
use crate::publish_door::{PublishDoor, Silent};
use crate::resources::MediaItems;
use crate::session_check::{SessionCheck, StaffRoutes};
use crate::{admin_api, admin_comments, api_version, content_api};

/// The document's title, the schemas every route shares and the ways a
/// key may be sent.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Khalid API",
        description = "The Content API and the Admin API of a Khalid blog."
    ),
    components(schemas(Problem, MediaItems)),
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

/// The staff routes and the session check of a site with staff accounts.
pub type Staff = (StaffRoutes, Arc<dyn SessionCheck>);

/// The Content API under `/api/content/v1/`, the Admin API under
/// `/api/admin/v1/`, and `/api/openapi.json`, with the document every
/// route declares (RULES.md BL-API-4). The document itself needs no key.
/// A post that goes live is announced to nobody; see [`router_with_door`].
pub fn router(
    content: Arc<dyn ContentStore>,
    posts: Arc<dyn PostStore>,
    keys: Arc<dyn KeyStore>,
) -> (Router, Document) {
    router_with_comments(content, posts, keys, Arc::new(NoComments))
}

/// [`router`] for a site that keeps readers' comments, moderated through
/// `comments` under `/api/admin/v1/comments/`. The document is the same.
pub fn router_with_comments(
    content: Arc<dyn ContentStore>,
    posts: Arc<dyn PostStore>,
    keys: Arc<dyn KeyStore>,
    comments: Arc<dyn CommentModeration>,
) -> (Router, Document) {
    let door = PublishDoor::new(posts, Arc::new(Silent));
    router_with_door_and_comments(content, Arc::new(door), keys, comments)
}

/// The same router, publishing through `door`, which `khalid serve` shares
/// with its scheduler so every post that goes live is announced.
pub fn router_with_door(
    content: Arc<dyn ContentStore>,
    door: Arc<PublishDoor>,
    keys: Arc<dyn KeyStore>,
) -> (Router, Document) {
    router_with_door_and_comments(content, door, keys, Arc::new(NoComments))
}

/// [`router_with_door`] for a site that keeps readers' comments, moderated
/// through `comments`. Without a media library, the media routes answer
/// that the site has none.
pub fn router_with_door_and_comments(
    content: Arc<dyn ContentStore>,
    door: Arc<PublishDoor>,
    keys: Arc<dyn KeyStore>,
    comments: Arc<dyn CommentModeration>,
) -> (Router, Document) {
    assemble(content, door, keys, comments, None, None)
}

/// [`router_with_door_and_comments`] with the site's media library under
/// `/api/admin/v1/media/`.
pub fn router_with_media(
    content: Arc<dyn ContentStore>,
    door: Arc<PublishDoor>,
    keys: Arc<dyn KeyStore>,
    comments: Arc<dyn CommentModeration>,
    media: MediaLibrary,
) -> (Router, Document) {
    assemble(content, door, keys, comments, Some(media), None)
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
    let door = Arc::new(PublishDoor::new(posts, Arc::new(Silent)));
    let comments = Arc::new(NoComments);
    let staff = Some((staff, sessions));
    assemble(content, door, keys, comments, None, staff)
}

/// [`router_with_door_and_comments`] with staff accounts.
/// [`router_with_staff`] says what `staff` and `sessions` are.
pub fn router_with_door_comments_and_staff(
    content: Arc<dyn ContentStore>,
    door: Arc<PublishDoor>,
    keys: Arc<dyn KeyStore>,
    comments: Arc<dyn CommentModeration>,
    staff: StaffRoutes,
    sessions: Arc<dyn SessionCheck>,
) -> (Router, Document) {
    assemble(content, door, keys, comments, None, Some((staff, sessions)))
}

/// [`router_with_media`] with staff accounts, as `khalid serve` runs it:
/// `staff` holds the staff routes and the session check that
/// [`router_with_staff`] describes.
pub fn router_with_media_and_staff(
    content: Arc<dyn ContentStore>,
    door: Arc<PublishDoor>,
    keys: Arc<dyn KeyStore>,
    comments: Arc<dyn CommentModeration>,
    media: MediaLibrary,
    staff: Staff,
) -> (Router, Document) {
    assemble(content, door, keys, comments, Some(media), Some(staff))
}

/// Every router above: the Content API, and the Admin API publishing
/// through `posts`, moderating `comments` and keeping `media`. With
/// `staff`, its routes join the Admin API and a staff session counts
/// wherever a key is needed.
fn assemble(
    content: Arc<dyn ContentStore>,
    posts: Arc<PublishDoor>,
    keys: Arc<dyn KeyStore>,
    comments: Arc<dyn CommentModeration>,
    media: Option<MediaLibrary>,
    staff: Option<Staff>,
) -> (Router, Document) {
    let (routes, sessions) = staff.unzip();
    let door = AdminDoor {
        keys: Arc::clone(&keys),
        sessions,
    };
    let mut admin = admin_api::routes(posts, door.clone())
        .merge(admin_comments::routes(comments, door.clone()))
        .merge(media_api::routes(media, door.clone()));
    if let Some(staff) = routes {
        let signed_in = staff
            .signed_in
            .route_layer(middleware::from_fn_with_state(door, require_admin_key));
        let staff = staff
            .open
            .merge(signed_in)
            .route_layer(middleware::from_fn(negotiate_version));
        admin = admin.merge(staff);
    }
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
