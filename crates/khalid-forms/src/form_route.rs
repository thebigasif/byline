//! The form block's address on a running site, answered as plain HTML.

use std::fmt;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::rejection::FormRejection;
use axum::extract::{ConnectInfo, DefaultBodyLimit, Form, Path, State};
use axum::http::{HeaderMap, HeaderValue, Response, StatusCode, header};
use axum::routing::{get, post};
use khalid_comments::{RateLimitError, RateLimits, reader_address};
use khalid_core::Clock;

use crate::answer_page::{self, Answer};
use crate::submission::{self, Problem, Reading};
use crate::{FormDelivery, FormId, FormStore, Submission};

/// The largest form body read. Thirty answers at their longest, written in
/// four-byte letters and percent-encoded, would not fit, but no reader
/// fills in that much.
const BODY_LIMIT: usize = 64 * 1024;

/// The forms the site's pages hold, as the binary finds them on each render.
pub trait SiteForms: fmt::Debug + Send + Sync {
    /// The address of the page form `form` is on, such as
    /// `/blog/contact/`, or `None` when no page has it.
    fn page_of(&self, form: &FormId) -> Option<String>;
}

/// What the form route needs.
#[derive(Debug)]
pub struct FormRoute<S> {
    /// Where answers are kept.
    pub store: Arc<S>,
    /// Where each kept answer is sent on.
    pub delivery: Arc<dyn FormDelivery>,
    /// The forms the site has.
    pub forms: Arc<dyn SiteForms>,
    /// The limits per network address.
    pub limits: RateLimits,
    /// When each answer is sent.
    pub clock: Box<dyn Clock>,
    /// The site's mount path, such as `/blog/`, where `forms/` starts.
    pub mount: String,
    /// The path of the site's stylesheet, which its pages link.
    pub stylesheet: String,
}

/// The routes `POST <mount>forms/{id}`, which keeps a form's answers, and
/// `GET <mount>forms/{id}/sent`, the thank-you page a kept answer is sent
/// on to with 303, so reloading it sends nothing twice. Bait goes there
/// too and is kept nowhere. Anything else answers a page saying why. It
/// needs the reader's address, so serve it with `ConnectInfo<SocketAddr>`.
pub fn form_router<S: FormStore + 'static>(route: FormRoute<S>) -> Router {
    let mount = route.mount.clone();
    Router::new()
        .route(&format!("{mount}forms/{{id}}"), post(receive::<S>))
        .route(&format!("{mount}forms/{{id}}/sent"), get(sent::<S>))
        .layer(DefaultBodyLimit::max(BODY_LIMIT))
        .with_state(Arc::new(route))
}

async fn receive<S: FormStore + 'static>(
    State(route): State<Arc<FormRoute<S>>>,
    Path(id): Path<String>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Result<Form<Vec<(String, String)>>, FormRejection>,
) -> Response<Body> {
    let Some((form, page)) = known(&route, &id) else {
        return answer(
            &route,
            StatusCode::NOT_FOUND,
            Answer::NotFound,
            &route.mount,
        );
    };
    let refuse = |status, why| answer(&route, status, why, &page);
    let Ok(Form(pairs)) = body else {
        let unreadable = Answer::Problem(Problem::Unreadable);
        return refuse(StatusCode::UNPROCESSABLE_ENTITY, unreadable);
    };
    let fields = match submission::read(pairs) {
        Reading::Answers(fields) => fields,
        Reading::Bait => return see_sent(&route.mount, &form),
        Reading::Problem(problem) => {
            return refuse(StatusCode::UNPROCESSABLE_ENTITY, Answer::Problem(problem));
        }
    };
    match route.limits.check(reader_address(peer.ip(), &headers)) {
        Ok(()) => {}
        Err(RateLimitError::Limited { retry_after }) => {
            let minutes = retry_after.as_secs().div_ceil(60).max(1);
            let mut response = refuse(StatusCode::TOO_MANY_REQUESTS, Answer::TooMany { minutes });
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, HeaderValue::from(minutes * 60));
            return response;
        }
        Err(error) => {
            tracing::warn!(%error, "a form's answers could not be checked against the limits");
            return refuse(StatusCode::SERVICE_UNAVAILABLE, Answer::Unavailable);
        }
    }
    let submission = Submission { form, page, fields };
    let at = route.clock.now();
    match route.store.insert(&submission, at).await {
        Ok(()) => {
            route.delivery.kept(&submission, at);
            see_sent(&route.mount, &submission.form)
        }
        Err(error) => {
            tracing::warn!(%error, form = %submission.form, "a form's answers could not be kept");
            let (status, why) = (StatusCode::SERVICE_UNAVAILABLE, Answer::Unavailable);
            answer(&route, status, why, &submission.page)
        }
    }
}

async fn sent<S: FormStore + 'static>(
    State(route): State<Arc<FormRoute<S>>>,
    Path(id): Path<String>,
) -> Response<Body> {
    match known(&route, &id) {
        Some((_, page)) => answer(&route, StatusCode::OK, Answer::Sent, &page),
        None => answer(
            &route,
            StatusCode::NOT_FOUND,
            Answer::NotFound,
            &route.mount,
        ),
    }
}

/// The form `id` names and the page it is on, when the site has it.
fn known<S>(route: &FormRoute<S>, id: &str) -> Option<(FormId, String)> {
    let form = FormId::try_new(id).ok()?;
    let page = route.forms.page_of(&form)?;
    Some((form, page))
}

/// 303 to the thank-you page of `form`.
fn see_sent(mount: &str, form: &FormId) -> Response<Body> {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::SEE_OTHER;
    if let Ok(location) = HeaderValue::from_str(&format!("{mount}forms/{form}/sent")) {
        response.headers_mut().insert(header::LOCATION, location);
    }
    response
}

/// The page for `why` with `status`, linking `back`, never cached, since
/// it answers one form.
fn answer<S>(route: &FormRoute<S>, status: StatusCode, why: Answer, back: &str) -> Response<Body> {
    let page = answer_page::page(why, back, &route.stylesheet);
    let mut response = Response::new(Body::from(page));
    *response.status_mut() = status;
    let headers = response.headers_mut();
    let html = HeaderValue::from_static("text/html; charset=utf-8");
    headers.insert(header::CONTENT_TYPE, html);
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}
