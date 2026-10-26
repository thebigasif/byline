//! The router, and the response each request gets.

use std::convert::Infallible;
use std::future::{Ready, ready};
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, HeaderValue, Method, Request, Response, StatusCode, header};
use bytes::Bytes;
use tower::service_fn;

use crate::clean_url::{Resolved, resolve};
use crate::{MediaType, Page, Pages, etag};

/// The methods a page answers.
const ALLOW: HeaderValue = HeaderValue::from_static("GET, HEAD");

/// Pages may be kept, but must be checked with their tag before reuse, so
/// an edited post shows on the next visit.
const REVALIDATE: HeaderValue = HeaderValue::from_static("no-cache");

/// The router that answers every request from `pages`.
///
/// `GET` and `HEAD` get the page, a redirect to its clean address, or the
/// not-found page with status 404. A page carries an `ETag` made from its
/// bytes, and a request whose `If-None-Match` names it gets 304 with no
/// body. Any other method gets 405.
pub fn router(pages: Pages) -> Router {
    let pages = Arc::new(pages);
    Router::new().fallback_service(service_fn(move |request: Request<Body>| {
        answer(&pages, &request)
    }))
}

/// Answers at once: every page is already in memory.
fn answer(pages: &Pages, request: &Request<Body>) -> Ready<Result<Response<Body>, Infallible>> {
    ready(Ok(respond(pages, request)))
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
        Resolved::Redirect(location) => redirect(&location),
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

/// The page, or 304 when the reader's copy is current.
fn page_response(page: &Page, request: &HeaderMap, send: SendBody) -> Response<Body> {
    let mut response = match page.etag() {
        Some(tag) if etag::is_current(request, tag) => status(StatusCode::NOT_MODIFIED),
        _ => file(StatusCode::OK, page.media_type(), page.body(), send),
    };
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, REVALIDATE);
    if let Some(tag) = page.etag() {
        headers.insert(header::ETAG, tag.clone());
    }
    response
}

fn file(code: StatusCode, media_type: MediaType, body: &Bytes, send: SendBody) -> Response<Body> {
    let payload = match send {
        SendBody::Yes => Body::from(body.clone()),
        SendBody::No => Body::empty(),
    };
    let mut response = Response::new(payload);
    *response.status_mut() = code;
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(media_type.as_header()),
    );
    headers.insert(header::CONTENT_LENGTH, HeaderValue::from(body.len()));
    response
}

fn redirect(location: &str) -> Response<Body> {
    let Ok(value) = HeaderValue::try_from(location) else {
        return status(StatusCode::NOT_FOUND);
    };
    let mut response = status(StatusCode::PERMANENT_REDIRECT);
    response.headers_mut().insert(header::LOCATION, value);
    response
}

fn status(code: StatusCode) -> Response<Body> {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = code;
    response
}
