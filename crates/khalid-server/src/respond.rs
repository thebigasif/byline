//! The router, and the response each request gets.

use std::convert::Infallible;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, HeaderValue, Method, Request, Response, StatusCode, header};
use bytes::Bytes;
use tower::service_fn;

use crate::clean_url::{Resolved, resolve};
use crate::compress::{self, Compressed};
use crate::{LivePages, MediaType, Page, Pages, SecurityHeaders, etag};

/// The methods a page answers.
const ALLOW: HeaderValue = HeaderValue::from_static("GET, HEAD");

/// Tells caches that a page's bytes depend on `Accept-Encoding`.
const ENCODING_VARIES: HeaderValue = HeaderValue::from_static("accept-encoding");

/// The `Content-Encoding` of a Brotli copy.
const BROTLI: HeaderValue = HeaderValue::from_static("br");

/// The router that answers every request from `pages`.
///
/// `GET` and `HEAD` get the page, a redirect to its clean address, a 301
/// for an address that moved, or the not-found page with status 404. A page carries an `ETag` made from its
/// bytes, and a request whose `If-None-Match` names it gets 304 with no
/// body. A page is cached as `no-cache`, and a [`Page::immutable`] file for
/// a year. Any other method gets 405. Every response, whatever its status,
/// carries `security`.
///
/// Each request reads the pages being served at that moment, so a
/// [`LivePages`] replaced after an edit shows the edit at once. While a
/// starting server serves its [`Pages::first`], a request for an address
/// they lack waits for the whole site rather than getting a 404.
pub fn router(pages: impl Into<LivePages>, security: SecurityHeaders) -> Router {
    let site = Arc::new((pages.into(), security));
    Router::new().fallback_service(service_fn(move |request: Request<Body>| {
        let site = Arc::clone(&site);
        async move {
            let (live, security) = &*site;
            let mut pages = live.current();
            if waits(&pages, &request) {
                pages = live.whole().await;
            }
            Ok::<_, Infallible>(answer(&pages, security, &request))
        }
    }))
}

/// Whether `request` asks a set of first pages for an address it lacks.
fn waits(pages: &Pages, request: &Request<Body>) -> bool {
    let uri = request.uri();
    !pages.is_whole()
        && matches!(*request.method(), Method::GET | Method::HEAD)
        && matches!(resolve(pages, uri.path(), uri.query()), Resolved::NotFound)
}

/// Answers at once from `pages`, already in memory.
fn answer(pages: &Pages, security: &SecurityHeaders, request: &Request<Body>) -> Response<Body> {
    let mut response = respond(pages, request);
    security.apply(response.headers_mut());
    response
}

fn respond(pages: &Pages, request: &Request<Body>) -> Response<Body> {
    let send_body = match *request.method() {
        Method::GET => SendBody::Yes,
        Method::HEAD => SendBody::No,
        _ => {
            let mut response = status(StatusCode::METHOD_NOT_ALLOWED);
            response.headers_mut().insert(header::ALLOW, ALLOW);
            return response;
        }
    };
    let uri = request.uri();
    match resolve(pages, uri.path(), uri.query()) {
        Resolved::Page(page) => page_response(page, request.headers(), send_body),
        Resolved::Redirect(location) => redirect(StatusCode::PERMANENT_REDIRECT, &location),
        Resolved::Moved(location) => redirect(StatusCode::MOVED_PERMANENTLY, &location),
        Resolved::NotFound => file(
            StatusCode::NOT_FOUND,
            MediaType::Html,
            pages.not_found(),
            send_body,
        ),
    }
}

/// Whether the response carries the body, which `HEAD` leaves out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SendBody {
    Yes,
    No,
}

/// The page, compressed when the browser takes Brotli, or 304 when the
/// reader's copy, plain or compressed, is current.
fn page_response(page: &Page, request: &HeaderMap, send: SendBody) -> Response<Body> {
    let brotli = page.brotli().filter(|_| compress::accepts_brotli(request));
    let tag = match brotli {
        Some(copy) => copy.etag(),
        None => page.etag(),
    };
    let tags = [page.etag(), page.brotli().and_then(Compressed::etag)];
    let current = tags
        .into_iter()
        .flatten()
        .any(|known| etag::is_current(request, known));
    let mut response = match (current, brotli, send) {
        (true, ..) => status(StatusCode::NOT_MODIFIED),
        (false, Some(copy), _) => file(StatusCode::OK, page.media_type(), copy.body(), send),
        (false, None, SendBody::No) => head(StatusCode::OK, page.media_type(), page.len()),
        (false, None, SendBody::Yes) => match page.body() {
            Ok(plain) => file(StatusCode::OK, page.media_type(), &plain, send),
            Err(error) => {
                tracing::error!(%error, "a page could not be sent plain");
                return status(StatusCode::INTERNAL_SERVER_ERROR);
            }
        },
    };
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, page.cache_control());
    headers.insert(header::VARY, ENCODING_VARIES);
    if let Some(tag) = tag {
        headers.insert(header::ETAG, tag.clone());
    }
    if brotli.is_some() && !current {
        headers.insert(header::CONTENT_ENCODING, BROTLI);
    }
    response
}

fn file(code: StatusCode, media_type: MediaType, body: &Bytes, send: SendBody) -> Response<Body> {
    match send {
        SendBody::Yes => sized(code, media_type, Body::from(body.clone()), body.len()),
        SendBody::No => head(code, media_type, body.len()),
    }
}

/// The headers of a `len`-byte file, without its body.
fn head(code: StatusCode, media_type: MediaType, len: usize) -> Response<Body> {
    sized(code, media_type, Body::empty(), len)
}

fn sized(code: StatusCode, media_type: MediaType, payload: Body, len: usize) -> Response<Body> {
    let mut response = Response::new(payload);
    *response.status_mut() = code;
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(media_type.as_header()),
    );
    headers.insert(header::CONTENT_LENGTH, HeaderValue::from(len));
    response
}

fn redirect(code: StatusCode, location: &str) -> Response<Body> {
    let Ok(value) = HeaderValue::try_from(location) else {
        return status(StatusCode::NOT_FOUND);
    };
    let mut response = status(code);
    response.headers_mut().insert(header::LOCATION, value);
    response
}

fn status(code: StatusCode) -> Response<Body> {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = code;
    response
}
