//! The consent form on a running site: the banner posts here, and the
//! reader's choice comes back as a cookie on a 303 to the page they were
//! reading (G45, ADR "consent is a form and a cookie").

use axum::Router;
use axum::body::Body;
use axum::extract::{DefaultBodyLimit, Form, FromRequest, Request, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::Response;
use axum::routing::post;
use khalid_core::Config;
use khalid_server::{
    CONSENT_COOKIE, HeldCode, LivePages, NONCE_SLOT, SecurityHeaders, router_with_consent,
};
use serde::Deserialize;

use crate::page_code::CONSENT_FOLDER;

/// The most the form may send: a choice and the address of a page.
const BODY_LIMIT: usize = 2 * 1024;

/// How long a choice is kept: 180 days, in seconds, so a reader is asked
/// about twice a year.
const KEPT_FOR: u32 = 180 * 24 * 60 * 60;

/// What the form route needs: where the blog lives, whether the cookie is
/// sent over HTTPS only, and the headers every answer carries.
#[derive(Debug, Clone)]
struct Answering {
    mount: String,
    secure: bool,
    security: SecurityHeaders,
}

/// The site's page router over `live`: as each reader's consent choice
/// says when the site's code waits for it, else as before.
pub(crate) fn pages(config: &Config, live: LivePages, security: SecurityHeaders) -> Router {
    match held(config) {
        Some(held) => router_with_consent(live, security, held),
        None => khalid_server::router(live, security),
    }
}

/// The site's code that waits for consent, as the page router puts it in
/// for a reader who agreed, or `None` when the site's code does not wait.
fn held(config: &Config) -> Option<HeldCode> {
    let code = config.code();
    if !code.waits_for_consent() {
        return None;
    }
    let head = code.head.as_ref().map(|head| head.to_html(NONCE_SLOT));
    let body = code.body.as_ref().map(|body| body.to_html(NONCE_SLOT));
    Some(HeldCode::new(
        head.unwrap_or_default(),
        body.unwrap_or_default(),
    ))
}

/// The route the consent form posts to, `<mount>_consent/`.
pub(crate) fn router(config: &Config, security: SecurityHeaders) -> Router {
    let mount = config.mount().to_owned();
    let route = format!("{mount}{CONSENT_FOLDER}/");
    let answering = Answering {
        secure: config.address().as_str().starts_with("https://"),
        mount,
        security,
    };
    Router::new()
        .route(
            &route,
            post(receive).layer(DefaultBodyLimit::max(BODY_LIMIT)),
        )
        .with_state(answering)
}

/// The consent form as a browser sends it.
#[derive(Debug, Deserialize)]
struct ConsentForm {
    choice: String,
    back: String,
}

/// Keeps the choice in a cookie and sends the reader back. A form posted
/// from another site gets 403, so no other site can choose for a reader.
async fn receive(State(answering): State<Answering>, request: Request) -> Response {
    let from_elsewhere = request
        .headers()
        .get("sec-fetch-site")
        .is_some_and(|site| site == "cross-site");
    let mut response = Response::new(Body::empty());
    answering.security.apply(response.headers_mut());
    if from_elsewhere {
        *response.status_mut() = StatusCode::FORBIDDEN;
        return response;
    }
    let Ok(Form(form)) = Form::<ConsentForm>::from_request(request, &()).await else {
        *response.status_mut() = StatusCode::BAD_REQUEST;
        return response;
    };
    *response.status_mut() = StatusCode::SEE_OTHER;
    let headers = response.headers_mut();
    let back = back_to(&answering.mount, &form.back);
    if let Ok(location) = HeaderValue::try_from(back) {
        headers.insert(header::LOCATION, location);
    }
    if let Some(cookie) = cookie(&answering, &form.choice) {
        headers.insert(header::SET_COOKIE, cookie);
    }
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

/// Where to send the reader: `back` when it is a page of the blog, else
/// the blog's home. Anything else could send the reader to another site.
fn back_to<'a>(mount: &'a str, back: &'a str) -> &'a str {
    let safe = back.starts_with(mount)
        && !back.contains("//")
        && back
            .bytes()
            .all(|b| b.is_ascii_graphic() && !matches!(b, b'\\' | b'"' | b'<' | b'>'));
    if safe { back } else { mount }
}

/// The cookie that keeps `choice`, or `None` for a choice the form never
/// sends.
fn cookie(answering: &Answering, choice: &str) -> Option<HeaderValue> {
    let value = match choice {
        "yes" | "no" => choice,
        _ => return None,
    };
    let secure = if answering.secure { "; Secure" } else { "" };
    let cookie = format!(
        "{CONSENT_COOKIE}={value}; Path={}; Max-Age={KEPT_FOR}; SameSite=Lax; HttpOnly{secure}",
        answering.mount
    );
    HeaderValue::try_from(cookie).ok()
}
