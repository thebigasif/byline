//! The newsletter's addresses on a running site: the form posts to
//! `subscribe`, and the links in emails open `subscribe/confirm` and
//! `unsubscribe`, all under the mount path.

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::rejection::FormRejection;
use axum::extract::{DefaultBodyLimit, Form, Query, State};
use axum::http::{HeaderName, HeaderValue, Response, StatusCode, header};
use axum::routing::{get, post};
use khalid_core::Slug;
use serde::Deserialize;

use crate::{
    BackPath, Confirmed, EmailAddress, Newsletter, Page, Pages, ReaderPass, SubscriberToken,
    Unsubscribed,
};

/// The newsletter a sign-up joins until task 5.7 adds several.
const DEFAULT_LIST: &str = "main";

/// The largest form body read. An address and a token fit many times over.
const LARGEST_FORM: usize = 4 * 1024;

/// Pages hold links with tokens and answers about one reader, so nothing
/// keeps them, no other site may frame them, and no link leaks the address.
/// The last two match what `khalid-server` sends with every other page.
const HEADERS: [(HeaderName, &str); 8] = [
    (header::CACHE_CONTROL, "no-store"),
    (header::REFERRER_POLICY, "no-referrer"),
    (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
    (
        header::CONTENT_SECURITY_POLICY,
        "default-src 'none'; form-action 'self'; frame-ancestors 'none'; base-uri 'none'",
    ),
    (HeaderName::from_static("x-robots-tag"), "noindex"),
    (header::CONTENT_TYPE, "text/html; charset=utf-8"),
    (
        HeaderName::from_static("permissions-policy"),
        "camera=(), microphone=(), geolocation=(), payment=(), usb=(), browsing-topics=()",
    ),
    (
        HeaderName::from_static("cross-origin-opener-policy"),
        "same-origin",
    ),
];

/// What every route reads. `newsletter` is `None` while the site cannot
/// send email, and then every page says sign-ups are not open.
#[derive(Debug)]
struct Shared {
    pages: Pages,
    newsletter: Option<Newsletter>,
}

/// The fields of the sign-up form. `homepage` is the trap a person never
/// sees; a program that fills it gets the same page and nothing happens.
#[derive(Debug, Deserialize)]
struct SignUpForm {
    email: Option<String>,
    homepage: Option<String>,
    /// The page of gated content the form was on.
    back: Option<String>,
}

/// The token of a link, from its query or from the page's one button, and
/// the page to come back to.
#[derive(Debug, Deserialize)]
struct Link {
    token: Option<String>,
    back: Option<String>,
}

/// The routes of the newsletter, with `pages` in the site's language.
/// The sign-up form of the `::newsletter` block posts to `/subscribe`, so
/// on a site under a mount path that address answers too.
pub fn router(pages: Pages, newsletter: Option<Newsletter>) -> Router {
    let mount = pages.mount().to_owned();
    let shared = Arc::new(Shared { pages, newsletter });
    let mut router = Router::new()
        .route(&format!("{mount}subscribe"), post(sign_up))
        .route(
            &format!("{mount}subscribe/confirm"),
            get(confirm_page).post(confirm),
        )
        .route(&format!("{mount}unsubscribe"), get(leave_page).post(leave));
    if mount != "/" {
        router = router.route("/subscribe", post(sign_up));
    }
    router
        .layer(DefaultBodyLimit::max(LARGEST_FORM))
        .with_state(shared)
}

async fn sign_up(
    State(shared): State<Arc<Shared>>,
    form: Result<Form<SignUpForm>, FormRejection>,
) -> Response<Body> {
    let Some(newsletter) = &shared.newsletter else {
        return page(&shared, StatusCode::SERVICE_UNAVAILABLE, Page::Closed);
    };
    let Ok(Form(form)) = form else {
        return page(&shared, StatusCode::UNPROCESSABLE_ENTITY, Page::WrongEmail);
    };
    if form.homepage.is_some_and(|trap| !trap.is_empty()) {
        return page(&shared, StatusCode::OK, Page::Check);
    }
    let email = form.email.as_deref().map(EmailAddress::parse);
    let Some(Ok(email)) = email else {
        return page(&shared, StatusCode::UNPROCESSABLE_ENTITY, Page::WrongEmail);
    };
    let Ok(list) = Slug::try_new(DEFAULT_LIST) else {
        return failed(&shared, &"the default newsletter name is not a slug");
    };
    let signed = match shared.back(form.back.as_deref()) {
        Some(back) => newsletter.sign_up_to_read(email, list, &back).await,
        None => newsletter.sign_up(email, list).await,
    };
    match signed {
        Ok(_) => page(&shared, StatusCode::OK, Page::Check),
        Err(error) => failed(&shared, &error),
    }
}

async fn confirm_page(
    State(shared): State<Arc<Shared>>,
    Query(link): Query<Link>,
) -> Response<Body> {
    let back = shared.back(link.back.as_deref());
    link_page(&shared, &link, |token| Page::Confirm {
        token,
        back: back.as_ref(),
    })
}

async fn leave_page(State(shared): State<Arc<Shared>>, Query(link): Query<Link>) -> Response<Body> {
    link_page(&shared, &link, Page::Leave)
}

/// The page a link opens: one button that posts its token. Opening it
/// changes nothing.
fn link_page<'a>(
    shared: &Shared,
    link: &'a Link,
    ask: impl FnOnce(&'a str) -> Page<'a>,
) -> Response<Body> {
    if shared.newsletter.is_none() {
        return page(shared, StatusCode::SERVICE_UNAVAILABLE, Page::Closed);
    }
    match link
        .token
        .as_deref()
        .filter(|token| SubscriberToken::parse(token).is_ok())
    {
        Some(token) => page(shared, StatusCode::OK, ask(token)),
        None => page(shared, StatusCode::NOT_FOUND, Page::Unknown),
    }
}

async fn confirm(
    State(shared): State<Arc<Shared>>,
    form: Result<Form<Link>, FormRejection>,
) -> Response<Body> {
    let Some(newsletter) = &shared.newsletter else {
        return page(&shared, StatusCode::SERVICE_UNAVAILABLE, Page::Closed);
    };
    let (token, back) = form
        .ok()
        .map(|Form(link)| (link.token.unwrap_or_default(), link.back))
        .unwrap_or_default();
    match newsletter.confirm(&token).await {
        Ok(Confirmed::Now(_) | Confirmed::Before) => {
            let back = shared.back(back.as_deref());
            let_in(&shared, newsletter, back.as_ref())
        }
        Ok(Confirmed::Expired) => page(&shared, StatusCode::GONE, Page::Expired),
        Ok(_) => page(&shared, StatusCode::NOT_FOUND, Page::Unknown),
        Err(error) => failed(&shared, &error),
    }
}

/// A confirmed reader: the reader cookie when gated content opens, then
/// a 303 to `back`, or the page that says they are subscribed.
fn let_in(shared: &Shared, newsletter: &Newsletter, back: Option<&BackPath>) -> Response<Body> {
    let mut response = match back {
        Some(back) => {
            let mut response = page(shared, StatusCode::SEE_OTHER, Page::Confirmed);
            if let Ok(location) = HeaderValue::try_from(back.as_str()) {
                response.headers_mut().insert(header::LOCATION, location);
            }
            response
        }
        None => page(shared, StatusCode::OK, Page::Confirmed),
    };
    let cookie = newsletter.pass().map(ReaderPass::cookie);
    if let Some(Ok(cookie)) = cookie.map(HeaderValue::try_from) {
        response.headers_mut().insert(header::SET_COOKIE, cookie);
    }
    response
}

impl Shared {
    /// `text` as a page on this blog to come back to, if it is one.
    fn back(&self, text: Option<&str>) -> Option<BackPath> {
        text.and_then(|text| BackPath::parse(self.pages.mount(), text))
    }
}

/// One click: the token comes from the page's button, or from the link's
/// query when a mail program posts to it (RFC 8058, one-click unsubscribe).
async fn leave(
    State(shared): State<Arc<Shared>>,
    Query(query): Query<Link>,
    form: Result<Form<Link>, FormRejection>,
) -> Response<Body> {
    let Some(newsletter) = &shared.newsletter else {
        return page(&shared, StatusCode::SERVICE_UNAVAILABLE, Page::Closed);
    };
    let token = query
        .token
        .or_else(|| form.ok().and_then(|Form(link)| link.token))
        .unwrap_or_default();
    match newsletter.unsubscribe(&token).await {
        Ok(Unsubscribed::Done) => page(&shared, StatusCode::OK, Page::Left),
        Ok(_) => page(&shared, StatusCode::NOT_FOUND, Page::Unknown),
        Err(error) => failed(&shared, &error),
    }
}

/// 503 with the page that says nothing was sent. The cause goes to the log,
/// which never holds an address or a token.
fn failed(shared: &Shared, why: &dyn std::fmt::Display) -> Response<Body> {
    tracing::warn!(%why, "a newsletter step failed");
    page(shared, StatusCode::SERVICE_UNAVAILABLE, Page::Failed)
}

fn page(shared: &Shared, status: StatusCode, page: Page<'_>) -> Response<Body> {
    let mut response = Response::new(Body::from(shared.pages.html(page)));
    *response.status_mut() = status;
    let headers = response.headers_mut();
    for (name, value) in HEADERS {
        headers.insert(name, HeaderValue::from_static(value));
    }
    response
}
