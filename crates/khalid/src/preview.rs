//! The page behind a preview link: a draft in the site's own design, for
//! anyone who has the link until it expires, and never for a search
//! engine (CTO answers 2026-10-27, section 4). It answers through the same
//! router as every page, so it carries the same security headers.

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::{Path, RawQuery, State};
use axum::http::{HeaderValue, Request, StatusCode, header};
use axum::response::Response;
use axum::routing::get;
use khalid_api::preview_link::{PREVIEW_FOLDER, PreviewError, PreviewSigner};
use khalid_api::publish_door::PublishDoor;
use khalid_core::{AbsoluteUrl, Config, Slug, SystemClock, UserMessage};
use khalid_db::{Database, SiteId};
use khalid_server::{MediaType, Page, PagePath, Pages, SecurityHeaders};
use tower::ServiceExt;

use crate::site_pages::SitePages;
use crate::{db_message, serve_message};

/// What the preview route reads.
struct Preview {
    door: Arc<PublishDoor>,
    pages: SitePages,
    config: Config,
    security: SecurityHeaders,
}

/// The signer of the site's preview links: the site's own key, from the
/// database at `file`, and the lifetime `site.toml` sets.
///
/// # Errors
///
/// Returns a message when the key cannot be read or made.
pub(crate) async fn signer(
    config: &Config,
    (db, site, file): (&Database, SiteId, &std::path::Path),
) -> Result<PreviewSigner, UserMessage> {
    let key = db.preview_key(site, &SystemClock).await;
    let key = key.map_err(|error| db_message::of(file, &error))?;
    let blog = format!(
        "{}{}",
        config.address().as_str().trim_end_matches('/'),
        config.mount()
    );
    let blog = AbsoluteUrl::parse(&blog).map_err(|_| serve_message::internal())?;
    let lifetime = config.preview_link_lifetime();
    Ok(PreviewSigner::new(&key, &blog, lifetime, SystemClock))
}

/// The route `<mount>preview/<id>/`, answering from `pages` the drafts
/// `door` holds, for links its signer made.
pub(crate) fn router(
    door: Arc<PublishDoor>,
    pages: SitePages,
    config: &Config,
    security: SecurityHeaders,
) -> Router {
    let route = format!("{}{PREVIEW_FOLDER}/{{id}}/", config.mount());
    let config = config.clone();
    let preview = Preview {
        door,
        pages,
        config,
        security,
    };
    Router::new()
        .route(&route, get(answer))
        .with_state(Arc::new(preview))
}

/// Shows the draft, or says why the link does not open it.
async fn answer(
    State(preview): State<Arc<Preview>>,
    Path(id): Path<String>,
    RawQuery(query): RawQuery,
    request: Request<Body>,
) -> Response {
    let path = request.uri().path().to_owned();
    let checked = match (preview.door.previews(), until_and_sig(query.as_deref())) {
        (Some(signer), Some((until, sig))) => signer.check(&id, until, sig),
        _ => Err(PreviewError::Forged),
    };
    let html = match checked {
        Ok(()) => draft(&preview, &id).await,
        Err(error) => Err(error),
    };
    let (status, page) = match html {
        Ok(html) => (StatusCode::OK, Some(noindex(&html))),
        Err(PreviewError::Expired) => (StatusCode::GONE, None),
        Err(_) => (StatusCode::NOT_FOUND, None),
    };
    let mut response = through_pages(&preview.security, &path, page, status).await;
    response.headers_mut().insert(
        "x-robots-tag",
        HeaderValue::from_static("noindex, nofollow"),
    );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response.headers_mut().remove(header::ETAG);
    *response.status_mut() = status;
    response
}

/// The `until` and `sig` of a preview link's query.
fn until_and_sig(query: Option<&str>) -> Option<(i64, &str)> {
    let (until, sig) = query?.split_once("&sig=")?;
    Some((until.strip_prefix("until=")?.parse().ok()?, sig))
}

/// The draft of post `id` as its page, or `Forged` when there is none to
/// show, so a guess learns nothing.
async fn draft(preview: &Arc<Preview>, id: &str) -> Result<String, PreviewError> {
    let (language, slug) = id.split_once('.').ok_or(PreviewError::Forged)?;
    let config = &preview.config;
    let locale = config.languages().iter().find(|l| l.as_str() == language);
    let (Some(locale), Ok(slug)) = (locale.cloned(), Slug::try_new_unicode(slug)) else {
        return Err(PreviewError::Forged);
    };
    let pages = preview.pages.clone();
    let rendered = tokio::task::spawn_blocking(move || pages.preview(&locale, &slug)).await;
    match rendered {
        Ok(Ok(Some(html))) => Ok(html),
        Ok(Err(message)) => {
            tracing::warn!(%message, "a draft could not be rendered for its preview link");
            Err(PreviewError::Forged)
        }
        _ => Err(PreviewError::Forged),
    }
}

/// `html` with a robots tag that keeps it out of search results.
fn noindex(html: &str) -> String {
    html.replacen(
        "<head>",
        "<head>\n<meta name=\"robots\" content=\"noindex, nofollow\">",
        1,
    )
}

/// Answers `path` through the site's page router, so the answer carries
/// every security header a page does: `page` when there is one, or the
/// message for `status`.
async fn through_pages(
    security: &SecurityHeaders,
    path: &str,
    page: Option<String>,
    status: StatusCode,
) -> Response {
    let pages = PagePath::try_new(path)
        .ok()
        .zip(page)
        .map(|(at, html)| Page::new(at, MediaType::Html, html))
        .into_iter()
        .collect();
    let message = if status == StatusCode::GONE {
        EXPIRED
    } else {
        UNKNOWN
    };
    let Ok(pages) = Pages::plain(pages, message) else {
        return Response::new(Body::from(UNKNOWN));
    };
    let inner = Request::get(path).body(Body::empty()).unwrap_or_default();
    match khalid_server::router(pages, security.clone())
        .oneshot(inner)
        .await
    {
        Ok(response) => response,
        Err(never) => match never {},
    }
}

/// The page for a link whose time has passed.
const EXPIRED: &str = "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
<meta name=\"robots\" content=\"noindex, nofollow\">\n<title>This preview link has expired</title>\n\
</head>\n<body>\n<main>\n<h1>This preview link has expired</h1>\n\
<p>Preview links stop working after a set time, so a forwarded link does not show a draft for ever.</p>\n\
<p>Ask the writer for a new link.</p>\n</main>\n</body>\n</html>\n";

/// The page for a link this site did not make, or a draft that is gone.
const UNKNOWN: &str = "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
<meta name=\"robots\" content=\"noindex, nofollow\">\n<title>This preview link does not open a draft</title>\n\
</head>\n<body>\n<main>\n<h1>This preview link does not open a draft</h1>\n\
<p>The link was changed, or the draft has since gone live or been deleted.</p>\n\
<p>Ask the writer for a new link, or look for the post on the blog.</p>\n</main>\n</body>\n</html>\n";
