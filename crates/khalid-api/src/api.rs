//! Both APIs behind one router, and the one `OpenAPI` document.

use std::sync::Arc;

use axum::middleware;
use axum::routing::get;
use axum::{Json, Router};
use khalid_core::SystemClock;
use utoipa::openapi::OpenApi as Document;
use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};
use utoipa::{Modify, OpenApi};
use utoipa_axum::router::OpenApiRouter;

use crate::admin_key::{AdminDoor, require_admin_key};
use crate::api_limits::ApiLimits;
use crate::api_version::negotiate_version;
use crate::comment_moderation::{CommentModeration, NoComments};
use crate::content_key::ContentDoor;
use crate::content_store::ContentStore;
use crate::entry_store::EntryStore;
use crate::keys::KeyStore;
use crate::media_api::{self, MediaLibrary};
use crate::post_check::PostChecks;
use crate::post_store::PostStore;
use crate::post_trash::PostTrash;
use crate::problem::Problem;
use crate::publish_door::{PublishDoor, Silent};
use crate::redirect_desk::RedirectDesk;
use crate::resources::{AdminAuthor, AdminCategory, AdminPage, AdminSnippet, MediaItems};
use crate::schema_template_store::SchemaTemplateStore;
use crate::session_check::{SessionCheck, StaffRoutes};
use crate::settings_store::SettingsStore;
use crate::tag_store::TagStore;
use crate::webhook_store::WebhookStore;
use crate::{
    admin_api, admin_api_keys, admin_audit, admin_authors, admin_categories, admin_check,
    admin_comments, admin_not_found, admin_pages, admin_redirect_csv, admin_redirects,
    admin_schema_templates, admin_site_settings, admin_snippets, admin_tags, admin_trash,
    admin_webhooks, api_version, content_api,
};

pub use crate::audit_desk::AuditDesk;

/// The document's title, the schemas every route shares and the ways a
/// key may be sent.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Khalid API",
        description = "The Content API and the Admin API of a Khalid blog. Each key, and each \
                       address that sends wrong keys, has a limit a minute; past it the answer \
                       is 429 with Retry-After."
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
/// Its redirect routes keep nothing; see [`router_with_redirects`].
pub fn router_with_door(
    content: Arc<dyn ContentStore>,
    door: Arc<PublishDoor>,
    keys: Arc<dyn KeyStore>,
) -> (Router, Document) {
    router_with_door_and_comments(content, door, keys, Arc::new(NoComments))
}

/// The same router, with the redirect routes under
/// `/api/admin/v1/redirects/` reading and writing through `redirects`.
pub fn router_with_redirects(
    content: Arc<dyn ContentStore>,
    door: Arc<PublishDoor>,
    keys: Arc<dyn KeyStore>,
    redirects: RedirectDesk,
) -> (Router, Document) {
    let desks = AdminDesks {
        redirects,
        ..AdminDesks::none()
    };
    assemble(content, door, keys, desks, None)
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
    let desks = AdminDesks {
        comments,
        ..AdminDesks::none()
    };
    assemble(content, door, keys, desks, None)
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
    let desks = AdminDesks {
        comments,
        media: Some(media),
        ..AdminDesks::none()
    };
    assemble(content, door, keys, desks, None)
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
    let staff = Some((staff, sessions));
    assemble(content, door, keys, AdminDesks::none(), staff)
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
    let desks = AdminDesks {
        comments,
        ..AdminDesks::none()
    };
    assemble(content, door, keys, desks, Some((staff, sessions)))
}

/// The Admin API's stores beside posts, as `khalid serve` holds them.
pub struct AdminDesks {
    /// Readers' comments, moderated under `/api/admin/v1/comments/`.
    pub comments: Arc<dyn CommentModeration>,
    /// The media library, or `None` when the media routes answer that the
    /// site has none.
    pub media: Option<MediaLibrary>,
    /// The stored redirects and the 404 log.
    pub redirects: RedirectDesk,
    /// The site's webhooks, or `None` when the webhook routes list none
    /// and answer that the site keeps none.
    pub webhooks: Option<Arc<dyn WebhookStore>>,
    /// The audit log every write that succeeds is kept in, or `None` when
    /// nothing is logged and the audit route lists nothing.
    pub audit: Option<AuditDesk>,
    /// Where deleted posts wait, or `None` when the trash routes list none.
    pub trash: Option<Arc<dyn PostTrash>>,
    /// The post checks behind the editor's SEO panel, or `None` when the
    /// check route answers that this server makes none.
    pub checks: Option<Arc<dyn PostChecks>>,
    /// The site's schema templates, or `None` when their routes answer
    /// that this server keeps none.
    pub schema_templates: Option<Arc<dyn SchemaTemplateStore>>,
    /// The site's pages, or `None` when the page routes answer that this
    /// server keeps none.
    pub pages: Option<Arc<dyn EntryStore<AdminPage>>>,
    /// The site's snippets, or `None` when the snippet routes answer that
    /// this server keeps none.
    pub snippets: Option<Arc<dyn EntryStore<AdminSnippet>>>,
    /// The site's authors, or `None` when the author routes answer that
    /// this server keeps none.
    pub authors: Option<Arc<dyn EntryStore<AdminAuthor>>>,
    /// The site's tags, or `None` when the tag routes answer that this
    /// server keeps none.
    pub tags: Option<Arc<dyn TagStore>>,
    /// The site's categories, or `None` when the category routes answer
    /// that this server keeps none.
    pub categories: Option<Arc<dyn EntryStore<AdminCategory>>>,
    /// The site's core settings, or `None` when the settings routes answer
    /// that this server keeps none.
    pub settings: Option<Arc<dyn SettingsStore>>,
}

impl std::fmt::Debug for AdminDesks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdminDesks")
            .field("media", &self.media)
            .field("redirects", &self.redirects)
            .finish_non_exhaustive()
    }
}

impl AdminDesks {
    /// No comments, no media library, no stored redirects, no webhooks, no
    /// audit log, no trash, no post checks, no schema templates, no pages,
    /// no snippets, no authors, no tags, no categories and no settings.
    pub fn none() -> Self {
        Self {
            comments: Arc::new(NoComments),
            media: None,
            redirects: RedirectDesk::none(),
            webhooks: None,
            audit: None,
            trash: None,
            checks: None,
            schema_templates: None,
            pages: None,
            snippets: None,
            authors: None,
            tags: None,
            categories: None,
            settings: None,
        }
    }
}

/// [`router_with_door`] keeping each deleted post in `trash`, under
/// `/api/admin/v1/trash/`. `trash` is usually the store behind `door`.
pub fn router_with_trash(
    content: Arc<dyn ContentStore>,
    door: Arc<PublishDoor>,
    keys: Arc<dyn KeyStore>,
    trash: Arc<dyn PostTrash>,
) -> (Router, Document) {
    let desks = AdminDesks {
        trash: Some(trash),
        ..AdminDesks::none()
    };
    assemble(content, door, keys, desks, None)
}

/// [`router_with_door`] keeping every write that succeeds in `audit`'s
/// log, which `/api/admin/v1/audit/` lists.
pub fn router_with_audit(
    content: Arc<dyn ContentStore>,
    door: Arc<PublishDoor>,
    keys: Arc<dyn KeyStore>,
    audit: AuditDesk,
) -> (Router, Document) {
    let desks = AdminDesks {
        audit: Some(audit),
        ..AdminDesks::none()
    };
    assemble(content, door, keys, desks, None)
}

/// [`router_with_door`] with every store in `desks`, and no staff.
pub fn router_with_desks(
    content: Arc<dyn ContentStore>,
    door: Arc<PublishDoor>,
    keys: Arc<dyn KeyStore>,
    desks: AdminDesks,
) -> (Router, Document) {
    assemble(content, door, keys, desks, None)
}

/// [`router_with_door`] with the webhook routes under
/// `/api/admin/v1/webhooks/` reading and writing through `webhooks`.
pub fn router_with_webhooks(
    content: Arc<dyn ContentStore>,
    door: Arc<PublishDoor>,
    keys: Arc<dyn KeyStore>,
    webhooks: Arc<dyn WebhookStore>,
) -> (Router, Document) {
    let desks = AdminDesks {
        webhooks: Some(webhooks),
        ..AdminDesks::none()
    };
    assemble(content, door, keys, desks, None)
}

/// The whole router as `khalid serve` runs it: every store in `desks`,
/// and `staff`, the staff routes and the session check that
/// [`router_with_staff`] describes.
pub fn router_for_serve(
    content: Arc<dyn ContentStore>,
    door: Arc<PublishDoor>,
    keys: Arc<dyn KeyStore>,
    desks: AdminDesks,
    staff: Staff,
) -> (Router, Document) {
    assemble(content, door, keys, desks, Some(staff))
}

/// Every router above: the Content API, and the Admin API publishing
/// through `posts` and keeping what `desks` hold. With `staff`, its routes
/// join the Admin API and a staff session counts wherever a key is needed.
fn assemble(
    content: Arc<dyn ContentStore>,
    posts: Arc<PublishDoor>,
    keys: Arc<dyn KeyStore>,
    desks: AdminDesks,
    staff: Option<Staff>,
) -> (Router, Document) {
    let (routes, sessions) = staff.unzip();
    let limits = Arc::new(ApiLimits::new(SystemClock));
    let door = AdminDoor {
        keys: Arc::clone(&keys),
        sessions,
        limits: Arc::clone(&limits),
        audit: desks.audit.clone(),
    };
    let content_door = ContentDoor {
        keys: Arc::clone(&keys),
        limits,
    };
    let redirects = desks.redirects;
    let mut admin = admin_api::routes(posts, door.clone())
        .merge(admin_audit::routes(desks.audit, door.clone()))
        .merge(admin_comments::routes(desks.comments, door.clone()))
        .merge(media_api::routes(desks.media, door.clone()))
        .merge(admin_not_found::routes(redirects.clone(), door.clone()))
        .merge(admin_redirect_csv::routes(redirects.clone(), door.clone()))
        .merge(admin_redirects::routes(redirects, door.clone()))
        .merge(admin_trash::routes(desks.trash, door.clone()))
        .merge(admin_webhooks::routes(desks.webhooks, door.clone()))
        .merge(admin_check::routes(desks.checks, door.clone()))
        .merge(admin_schema_templates::routes(
            desks.schema_templates,
            door.clone(),
        ))
        .merge(admin_pages::routes(desks.pages, door.clone()))
        .merge(admin_snippets::routes(desks.snippets, door.clone()))
        .merge(admin_authors::routes(desks.authors, door.clone()))
        .merge(admin_tags::routes(desks.tags, door.clone()))
        .merge(admin_categories::routes(desks.categories, door.clone()))
        .merge(admin_site_settings::routes(desks.settings, door.clone()))
        .merge(admin_api_keys::routes(keys, door.clone()));
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
        .nest(
            content_api::BASE,
            content_api::routes(content, content_door),
        )
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
