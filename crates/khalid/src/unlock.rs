//! Password pages on a running site: the form a locked post shows posts
//! here, a right password gets a signed cookie and the post, and a request
//! with that cookie is answered from the post's full page, which no one
//! reaches any other way (ADR "a password page is a plain form and a
//! signed cookie").

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::{DefaultBodyLimit, Form, Request, State};
use axum::http::{HeaderValue, Method, StatusCode, Uri, header};
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::post;
use khalid_api::post_password::password_matches;
use khalid_api::unlock::UnlockSigner;
use khalid_core::{Clock, Config, PasswordHash};
use khalid_newsletter::ReaderPass;
use percent_encoding::percent_decode_str;
use serde::Deserialize;
use tower::ServiceExt;

use crate::build_pages::LanguagePages;
use crate::post_lock::{Key, UNLOCK_FOLDER, UNLOCKED_FOLDER, hidden};

/// The most a form may send: an address and a password.
const BODY_LIMIT: usize = 4 * 1024;

/// The most wrong passwords one post takes in a minute, so guessing is
/// slow even before Argon2id makes each guess slow.
const TRIES_PER_MINUTE: u32 = 10;

/// The largest locked page read back to show its alert.
const PAGE_LIMIT: usize = 4 * 1024 * 1024;

/// The password posts and the posts with gated content of the last full
/// render: each one's address, its file and what opens it. Cloning shares
/// it.
#[derive(Debug, Clone, Default)]
pub(crate) struct Gate(Arc<Mutex<BTreeMap<String, (PathBuf, Key)>>>);

impl Gate {
    /// Takes the locked posts of `languages` in place of the old ones.
    pub(crate) fn replace(&self, languages: &[LanguagePages]) {
        let posts = languages.iter().flat_map(|language| &language.unlocked);
        let entries = posts.filter_map(|unlocked| {
            let source = unlocked.page.source.clone()?;
            Some((
                unlocked.page.meta.path.clone(),
                (source, unlocked.key.clone()),
            ))
        });
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = entries.collect();
    }

    /// Whether the post file `source` has a full page kept now.
    pub(crate) fn guards(&self, source: &std::path::Path) -> bool {
        let posts = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        posts.values().any(|(file, _)| file == source)
    }

    /// The hash of the password of the post at `path`, if it has one.
    fn hash(&self, path: &str) -> Option<PasswordHash> {
        match self.key(path) {
            Some(Key::Password(hash)) => Some(hash),
            _ => None,
        }
    }

    /// What opens the full page of the post at `path`, if it has one.
    fn key(&self, path: &str) -> Option<Key> {
        let posts = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        posts.get(path).map(|(_, key)| key.clone())
    }
}

/// What the form route and the cookie check read.
struct Unlock {
    gate: Gate,
    signer: UnlockSigner,
    /// Checks the reader cookie of confirmed subscribers.
    reader: ReaderPass,
    /// The site's page router, which answers with every security header.
    pages: Router,
    clock: Box<dyn Clock>,
    /// Wrong passwords per post this minute.
    tries: Mutex<BTreeMap<String, (i64, u32)>>,
}

/// `pages`, the site's page router, with the password form's route at
/// `<mount>_unlock/` and the cookie checks in front of every page: the
/// password cookie `signer` made, and the reader cookie `reader` made.
pub(crate) fn router(
    config: &Config,
    gate: Gate,
    (signer, reader): (UnlockSigner, ReaderPass),
    pages: Router,
    clock: impl Clock + 'static,
) -> Router {
    let route = format!("{}{UNLOCK_FOLDER}/", config.mount());
    let unlock = Arc::new(Unlock {
        gate,
        signer,
        reader,
        pages: pages.clone(),
        clock: Box::new(clock),
        tries: Mutex::default(),
    });
    let checked = pages.layer(middleware::from_fn_with_state(Arc::clone(&unlock), let_in));
    Router::new()
        .route(
            &route,
            post(receive).layer(DefaultBodyLimit::max(BODY_LIMIT)),
        )
        .with_state(unlock)
        .merge(checked)
}

/// The password form as a browser sends it.
#[derive(Debug, Deserialize)]
struct UnlockForm {
    path: String,
    password: String,
}

/// Checks a password: right gets the cookie and a 303 back to the post,
/// wrong gets the locked page again with its alert shown.
async fn receive(State(unlock): State<Arc<Unlock>>, Form(form): Form<UnlockForm>) -> Response {
    let Some(hash) = unlock.gate.hash(&form.path) else {
        return through(&unlock.pages, "/_khalid/no-such-post/").await;
    };
    if !unlock.may_try(&form.path) {
        return wrong(&unlock.pages, &form.path, StatusCode::TOO_MANY_REQUESTS).await;
    }
    let checked = hash.clone();
    let password = form.password;
    let right = tokio::task::spawn_blocking(move || password_matches(&checked, &password)).await;
    if !matches!(right, Ok(true)) {
        unlock.count_wrong(&form.path);
        return wrong(&unlock.pages, &form.path, StatusCode::UNAUTHORIZED).await;
    }
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::SEE_OTHER;
    let cookie = unlock.signer.cookie(&form.path, &hash);
    let headers = response.headers_mut();
    if let (Ok(location), Ok(cookie)) = (
        HeaderValue::try_from(&form.path),
        HeaderValue::try_from(cookie),
    ) {
        headers.insert(header::LOCATION, location);
        headers.insert(header::SET_COOKIE, cookie);
    }
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

impl Unlock {
    /// Whether the post at `path` takes another try this minute.
    fn may_try(&self, path: &str) -> bool {
        let minute = self.clock.now().timestamp() / 60;
        let tries = self.tries.lock().unwrap_or_else(PoisonError::into_inner);
        tries
            .get(path)
            .is_none_or(|&(at, count)| at != minute || count < TRIES_PER_MINUTE)
    }

    /// Counts a wrong password for the post at `path`.
    fn count_wrong(&self, path: &str) {
        let minute = self.clock.now().timestamp() / 60;
        let mut tries = self.tries.lock().unwrap_or_else(PoisonError::into_inner);
        let entry = tries.entry(path.to_owned()).or_insert((minute, 0));
        *entry = if entry.0 == minute {
            (minute, entry.1 + 1)
        } else {
            (minute, 1)
        };
    }
}

/// Answers a request for a page: a hidden full page asked for from
/// outside gets 404, a password post asked for with its cookie, or a post
/// with gated content asked for with the reader cookie, gets its full
/// page, never kept by any shared cache, and the rest go on as they came.
/// A locked page says it varies by cookie, so no cache hands it to a
/// reader who holds one.
async fn let_in(State(unlock): State<Arc<Unlock>>, mut request: Request, next: Next) -> Response {
    // The pages are looked up by the decoded path, so the checks read it
    // decoded too: /%5Funlocked/ is /_unlocked/.
    let path = percent_decode_str(request.uri().path())
        .decode_utf8_lossy()
        .into_owned();
    if path.starts_with(&format!("/{UNLOCKED_FOLDER}")) {
        *request.uri_mut() = Uri::from_static("/_khalid/no-such-page/");
        return next.run(request).await;
    }
    let reads = matches!(*request.method(), Method::GET | Method::HEAD);
    let cookies = request
        .headers()
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok());
    let key = unlock.gate.key(&path);
    let locked = key.is_some();
    let opens = match (reads, cookies, key) {
        (true, Some(cookies), Some(Key::Password(hash))) => {
            unlock.signer.opens(cookies, &path, &hash)
        }
        (true, Some(cookies), Some(Key::Reader)) => unlock.reader.admits(cookies),
        _ => false,
    };
    if !opens {
        let mut response = next.run(request).await;
        if locked {
            let vary = HeaderValue::from_static("cookie");
            response.headers_mut().append(header::VARY, vary);
        }
        return response;
    }
    if let Ok(uri) = Uri::try_from(hidden(&path)) {
        *request.uri_mut() = uri;
    }
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-store"),
    );
    headers.insert(header::VARY, HeaderValue::from_static("cookie"));
    response
}

/// The locked page at `path` with its alert shown, with `status`.
async fn wrong(pages: &Router, path: &str, status: StatusCode) -> Response {
    let response = through(pages, path).await;
    let (mut parts, body) = response.into_parts();
    let Ok(bytes) = to_bytes(body, PAGE_LIMIT).await else {
        return Response::from_parts(parts, Body::empty());
    };
    let html = String::from_utf8_lossy(&bytes).replacen(
        "class=\"unlock-error\" role=\"alert\" hidden>",
        "class=\"unlock-error\" role=\"alert\">",
        1,
    );
    parts.status = status;
    parts.headers.remove(header::CONTENT_LENGTH);
    parts.headers.remove(header::ETAG);
    parts
        .headers
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Response::from_parts(parts, Body::from(html))
}

/// `pages`' plain answer for a GET of `path`, with its security headers.
async fn through(pages: &Router, path: &str) -> Response {
    let request = Request::get(path).body(Body::empty()).unwrap_or_default();
    match pages.clone().oneshot(request).await {
        Ok(response) => response,
        Err(never) => match never {},
    }
}
