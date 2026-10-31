//! The Content API and the Admin API as `khalid serve` wires them to one
//! site: every Admin API desk backed by the site's files and database. Kept
//! apart from `serve.rs` so that file stays under 600 lines (BL-SIZE-1).

use std::path::Path;
use std::sync::Arc;

use axum::Router;
use khalid_api::analytics_desk::AnalyticsDesk;
use khalid_api::api::{self, AdminDesks, AuditDesk};
use khalid_api::comment_moderation::CommentModeration;
use khalid_api::media_api::MediaLibrary;
use khalid_api::publish_door::PublishDoor;
use khalid_api::redirect_desk::RedirectDesk;
use khalid_api::resources::{AdminAuthor, AdminPage, AdminSnippet};
use khalid_api::session_check::SessionCheck;
use khalid_auth::SiteDirectory;
use khalid_core::{Config, SystemClock};
use khalid_db::{AnalyticsTable, AuditTable, Database, KeyTable, SiteId, WebhookTable};
use khalid_newsletter::Mailer;

use crate::site_categories::SiteCategories;
use crate::site_content::LiveContent;
use crate::site_entries::SiteEntries;
use crate::site_posts::SitePosts;
use crate::site_schema_templates::SiteSchemaTemplates;
use crate::site_settings::SiteSettings;
use crate::site_tags::SiteTags;
use crate::{draft_comment_mail, late_api, staff};

/// The Content API and the Admin API as `khalid serve` runs them, with the
/// comment moderation, media library, stored redirects, webhooks and staff
/// accounts of site `id`, the post checks, schema templates, pages,
/// snippets, authors, tags, categories and settings of the site in `site`,
/// the super-admin routes over `panel`, and comments on drafts emailed
/// through `mailer` when the site sends mail.
pub(crate) fn api_router(
    (content, door, posts): (LiveContent, Arc<PublishDoor>, SitePosts),
    (comments, media, redirects): (Arc<dyn CommentModeration>, MediaLibrary, RedirectDesk),
    (database, id, (site, config)): (&Database, SiteId, (&Path, &Config)),
    (panel, mailer): (Arc<dyn SiteDirectory>, Option<Arc<dyn Mailer>>),
) -> Router {
    let accounts = staff::accounts(database, id, config);
    let sessions: Arc<dyn SessionCheck> = Arc::new(accounts.clone());
    let routes = khalid_auth::staff_routes(accounts.clone());
    let staff = (khalid_auth::with_sites(routes, accounts, panel), sessions);
    let desks = AdminDesks {
        comments,
        media: Some(media),
        redirects,
        webhooks: Some(Arc::new(WebhookTable::new(
            database.clone(),
            id,
            SystemClock,
        ))),
        trash: Some(Arc::new(posts)),
        audit: Some(AuditDesk::new(
            Arc::new(AuditTable::new(database.clone(), id)),
            SystemClock,
        )),
        checks: Some(Arc::new(SitePosts::new(site, config.clone(), SystemClock))),
        schema_templates: Some(Arc::new(SiteSchemaTemplates::new(site, config))),
        pages: Some(Arc::new(SiteEntries::<AdminPage>::new(site, config))),
        snippets: Some(Arc::new(SiteEntries::<AdminSnippet>::new(site, config))),
        authors: Some(Arc::new(SiteEntries::<AdminAuthor>::new(site, config))),
        tags: Some(Arc::new(SiteTags::new(site, config))),
        categories: Some(Arc::new(SiteCategories::new(site, config))),
        settings: Some(Arc::new(SiteSettings::new(site, config))),
        analytics: AnalyticsDesk::new(
            Arc::new(AnalyticsTable::new(database.clone(), id)),
            Arc::new(SystemClock),
        ),
        draft_comments: Some(draft_comment_mail::desk((database, id, config), mailer)),
    };
    let keys = Arc::new(KeyTable::new(database.clone(), id, SystemClock));
    let (api, _) = api::router_for_serve(Arc::new(content), door, keys, desks, staff);
    api
}

/// What [`api_router`] is built from: its four argument groups.
pub(crate) type ApiParts<'a> = (
    (LiveContent, Arc<PublishDoor>, SitePosts),
    (Arc<dyn CommentModeration>, MediaLibrary, RedirectDesk),
    (&'a Database, SiteId, (&'a Path, &'a Config)),
    (Arc<dyn SiteDirectory>, Option<Arc<dyn Mailer>>),
);

/// `app` with the router of [`api_router`] and git mode's push `hook`
/// answering under `/api/`, built on a blocking thread beside the first
/// answer (ADR "the APIs are built beside the first answer"). An address
/// under `/api/` the APIs lack goes to `app`, as it did when the routers
/// were merged before the first answer.
pub(crate) fn with_apis(
    app: Router,
    (apis, desks, at, helpers): ApiParts<'_>,
    hook: Router,
) -> Router {
    let (database, id, (site, config)) = at;
    let (database, site, config) = (database.clone(), site.to_path_buf(), config.clone());
    let rest = app.clone();
    late_api::router(app, move || {
        let at = (&database, id, (site.as_path(), &config));
        api_router(apis, desks, at, helpers)
            .merge(hook)
            .fallback_service(rest)
    })
}
