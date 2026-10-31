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
use crate::consent::{self, Choice, HeldCode};
use crate::page::NONCE_SLOT;
use crate::security::fresh_nonce;
use crate::{LivePages, MediaType, Page, Pages, SecurityHeaders, etag};

/// The methods a page answers.
const ALLOW: HeaderValue = HeaderValue::from_static("GET, HEAD");

/// Tells caches that a page's bytes depend on `Accept-Encoding`.
const ENCODING_VARIES: HeaderValue = HeaderValue::from_static("accept-encoding");

/// Tells caches that a page's bytes depend on `Accept-Encoding` and on the
/// reader's consent cookie.
const ENCODING_AND_COOKIE_VARY: HeaderValue = HeaderValue::from_static("accept-encoding, cookie");

/// Tells caches that a page made for one reader's choice depends on their
/// cookie.
const COOKIE_VARIES: HeaderValue = HeaderValue::from_static("cookie");

/// A page made for one reader's choice, kept by no cache at all.
const PRIVATE: HeaderValue = HeaderValue::from_static("private, no-store");

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
    answering(pages.into(), security, None)
}

/// The [`router`] for a site whose own code waits for each reader's
/// consent: `held` goes into the HTML pages of a reader whose
/// [`CONSENT_COOKIE`](crate::CONSENT_COOKIE) says yes.
///
/// A reader who has not answered gets each page as it was rendered, with
/// its tag and its Brotli copy, and no held code. A reader who answered
/// gets HTML pages made for them alone: without the banner, with the form
/// to change their choice, kept by no cache, and, when they said yes,
/// with the code and a policy carrying its nonce.
pub fn router_with_consent(
    pages: impl Into<LivePages>,
    security: SecurityHeaders,
    held: HeldCode,
) -> Router {
    answering(pages.into(), security, Some(held))
}

/// The router of `live`, with `held` when the site asks for consent.
fn answering(live: LivePages, security: SecurityHeaders, held: Option<HeldCode>) -> Router {
    let site = Arc::new((live, security, held));
    Router::new().fallback_service(service_fn(move |request: Request<Body>| {
        let site = Arc::clone(&site);
        async move {
            let (live, security, held) = &*site;
            let mut pages = live.current();
            if waits(&pages, &request) {
                pages = live.whole().await;
            }
            let consent = held
                .as_ref()
                .map(|held| (Choice::of(request.headers()), held));
            Ok::<_, Infallible>(answer(&pages, security, &request, consent))
        }
    }))
}

/// A reader's choice and the code it lets in, on a site that asks.
type Consent<'held> = Option<(Choice, &'held HeldCode)>;

/// Whether `request` asks a set of first pages for an address it lacks.
fn waits(pages: &Pages, request: &Request<Body>) -> bool {
    let uri = request.uri();
    !pages.is_whole()
        && matches!(*request.method(), Method::GET | Method::HEAD)
        && matches!(resolve(pages, uri.path(), uri.query()), Resolved::NotFound)
}

/// Answers at once from `pages`, already in memory, as the reader who made
/// `consent`'s choice sees them.
fn answer(
    pages: &Pages,
    security: &SecurityHeaders,
    request: &Request<Body>,
    consent: Consent<'_>,
) -> Response<Body> {
    let mut response = respond(pages, request, consent);
    let html = response.headers().get(header::CONTENT_TYPE).is_none() || is_html(&response);
    if html && !pages.preloads().is_empty() {
        let link = pages
            .preloads()
            .iter()
            .map(|href| format!("<{href}>; rel=preload; as=style"))
            .collect::<Vec<_>>()
            .join(", ");
        if let Ok(value) = HeaderValue::from_str(&link) {
            response.headers_mut().insert(header::LINK, value);
        }
    }
    match response.extensions_mut().remove::<FilledNonce>() {
        Some(FilledNonce(nonce)) => security.apply_with_head_code(response.headers_mut(), &nonce),
        None => security.apply(response.headers_mut()),
    }
    response
}

/// Whether `response` is an HTML answer, which a stylesheet preload helps.
fn is_html(response: &Response<Body>) -> bool {
    response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.to_ascii_lowercase().starts_with("text/html"))
}

/// The nonce a page's [`NONCE_SLOT`] was filled with, which its policy
/// must carry.
#[derive(Debug, Clone)]
struct FilledNonce(String);

/// `plain`, an HTML page, made for this request alone: changed for the
/// reader's choice on a site that asks for consent, with its nonce slot
/// filled. The policy carries the nonce only while a slot is left to use
/// it. Without random bytes the slot is emptied and the policy allows no
/// script, which is stricter, never looser.
fn personal(
    code: StatusCode,
    (media_type, plain): (MediaType, &Bytes),
    send: SendBody,
    consent: Consent<'_>,
) -> Response<Body> {
    let text = String::from_utf8_lossy(plain);
    let text = match consent {
        Some((choice, held)) => consent::chosen(&text, choice, held),
        None => text.into_owned(),
    };
    let nonce = fresh_nonce().filter(|_| text.contains(NONCE_SLOT));
    let html = text.replace(NONCE_SLOT, nonce.as_deref().unwrap_or(""));
    let mut response = file(code, media_type, &Bytes::from(html), send);
    if consent.is_some() {
        let headers = response.headers_mut();
        headers.insert(header::CACHE_CONTROL, PRIVATE);
        headers.insert(header::VARY, COOKIE_VARIES);
    }
    if let Some(nonce) = nonce {
        response.extensions_mut().insert(FilledNonce(nonce));
    }
    response
}

/// A page with a nonce slot, or one asked for by a reader who answered,
/// made for this request alone and never kept.
fn per_request(page: &Page, send: SendBody, consent: Consent<'_>) -> Response<Body> {
    let plain = match page.body() {
        Ok(plain) => plain,
        Err(error) => {
            tracing::error!(%error, "a page could not be sent plain");
            return status(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    let mut response = personal(StatusCode::OK, (page.media_type(), &plain), send, consent);
    if consent.is_none() {
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, page.cache_control());
    }
    response
}

fn respond(pages: &Pages, request: &Request<Body>, consent: Consent<'_>) -> Response<Body> {
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
        Resolved::Page(page) => page_response(page, request.headers(), send_body, consent),
        Resolved::Redirect(location) => redirect(StatusCode::PERMANENT_REDIRECT, &location),
        Resolved::Moved(location) => redirect(StatusCode::MOVED_PERMANENTLY, &location),
        Resolved::NotFound if answered(consent) => personal(
            StatusCode::NOT_FOUND,
            (MediaType::Html, pages.not_found()),
            send_body,
            consent,
        ),
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

/// Whether the reader has answered the banner of a site that asks, so its
/// HTML pages are made for them alone.
fn answered(consent: Consent<'_>) -> bool {
    consent.is_some_and(|(choice, _)| choice != Choice::Unasked)
}

/// The page, compressed when the browser takes Brotli, or 304 when the
/// reader's copy, plain or compressed, is current.
fn page_response(
    page: &Page,
    request: &HeaderMap,
    send: SendBody,
    consent: Consent<'_>,
) -> Response<Body> {
    let html = page.media_type() == MediaType::Html;
    if page.fills_nonce() || (html && answered(consent)) {
        return per_request(page, send, consent);
    }
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
    let vary = if html && consent.is_some() {
        ENCODING_AND_COOKIE_VARY
    } else {
        ENCODING_VARIES
    };
    headers.insert(header::VARY, vary);
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
