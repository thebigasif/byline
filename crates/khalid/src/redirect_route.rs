//! Stored redirects on the running site (KH-SEO-8): a layer over the page
//! router that answers an address with no page from the redirect table.
//!
//! Pages and the aliases in front matter answer first, so a stored rule
//! only ever catches an address that would be a 404. The answer keeps the
//! 404's security headers. A move to a path an alias has moved again is
//! sent straight to the end, so a reader takes one step.

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{HeaderValue, Method, Response, StatusCode, header};
use axum::middleware::{self, Next};
use khalid_api::redirect_desk::RedirectDesk;
use khalid_core::SystemClock;
use khalid_db::{Database, NotFoundTable, RedirectRows, SiteId};
use khalid_seo::{Destination, Redirect, UrlPath};
use khalid_server::LivePages;
use percent_encoding::{AsciiSet, CONTROLS, percent_decode_str, utf8_percent_encode};

use crate::live_redirects::LiveRedirects;
use crate::site_addresses::LiveAddresses;

/// The characters encoded when a path goes into a `Location` header, as
/// the page router encodes them.
const PATH: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'`')
    .add(b'{')
    .add(b'}');

/// What the layer reads: the stored redirects, the pages for their
/// aliases, and the folder the blog answers under.
#[derive(Debug)]
struct Redirecting {
    redirects: LiveRedirects,
    pages: LivePages,
    mount: String,
}

/// The site's stored redirects in `db`: the desk the Admin
/// API's redirect routes use, and `router`, which serves the `live` pages
/// under `mount`, with stored redirects answering in place of its 404s.
pub(crate) async fn start(
    (db, site): (&Database, SiteId),
    (router, live, mount): (Router, LivePages, &str),
    addresses: LiveAddresses,
) -> (RedirectDesk, Router) {
    let redirects = LiveRedirects::load(Arc::new(RedirectRows::new(db.clone(), site))).await;
    let desk = RedirectDesk::new(
        Arc::new(redirects.clone()),
        Arc::new(NotFoundTable::new(db.clone(), site)),
        Arc::new(addresses),
        Arc::new(SystemClock),
    );
    let redirecting = Redirecting {
        redirects,
        pages: live,
        mount: mount.to_owned(),
    };
    let state = Arc::new(redirecting);
    (
        desk,
        router.layer(middleware::from_fn_with_state(state, answer)),
    )
}

/// The page's own answer, unless it is a 404 for a `GET` or `HEAD` under
/// the mount folder that a stored redirect matches.
async fn answer(
    State(redirecting): State<Arc<Redirecting>>,
    request: Request,
    next: Next,
) -> Response<Body> {
    let asked = matches!(*request.method(), Method::GET | Method::HEAD)
        .then(|| asked_path(&redirecting.mount, &request))
        .flatten();
    let query = request.uri().query().map(ToOwned::to_owned);
    let response = next.run(request).await;
    let Some(path) = asked.filter(|_| response.status() == StatusCode::NOT_FOUND) else {
        return response;
    };
    let Some(hit) = redirecting.redirects.table().resolve(&path) else {
        return response;
    };
    redirected(response, &hit.answer, &redirecting, query.as_deref())
}

/// The decoded path of `request` when it is a clean path inside `mount`
/// and not an API address.
pub(crate) fn asked_path(mount: &str, request: &Request) -> Option<UrlPath> {
    let path = percent_decode_str(request.uri().path())
        .decode_utf8()
        .ok()?;
    let inside = path.starts_with(mount) && !path.starts_with("/api/");
    inside.then(|| UrlPath::try_new(&path).ok()).flatten()
}

/// The 404 `response` turned into `answer`: a move with its `Location`, or
/// a 410 or 451 that keeps the not-found page as its body.
fn redirected(
    mut response: Response<Body>,
    answer: &Redirect,
    redirecting: &Redirecting,
    query: Option<&str>,
) -> Response<Body> {
    let Ok(status) = StatusCode::from_u16(answer.status()) else {
        return response;
    };
    *response.status_mut() = status;
    let Some(to) = answer.location() else {
        return response;
    };
    let location = match to {
        Destination::Path(path) => {
            let pages = redirecting.pages.current();
            let end = pages
                .moved(path.as_str())
                .map_or(path.as_str(), |to| to.as_str());
            let mut location = utf8_percent_encode(end, PATH).to_string();
            if let Some(query) = query {
                location = format!("{location}?{query}");
            }
            location
        }
        Destination::External(url) => url.to_string(),
        _ => return response,
    };
    let Ok(location) = HeaderValue::from_str(&location) else {
        tracing::warn!(%location, "a redirect's address cannot be sent as a header");
        *response.status_mut() = StatusCode::NOT_FOUND;
        return response;
    };
    let headers = response.headers_mut();
    for name in [header::CONTENT_TYPE, header::CONTENT_LENGTH, header::ETAG] {
        headers.remove(name);
    }
    headers.insert(header::LOCATION, location);
    *response.body_mut() = Body::empty();
    response
}
