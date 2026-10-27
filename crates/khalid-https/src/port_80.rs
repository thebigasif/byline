//! What plain HTTP on port 80 answers when the site serves HTTPS.

use axum::Router;
use axum::extract::{Path, State};
use axum::http::header::{CONTENT_TYPE, LOCATION};
use axum::http::uri::PathAndQuery;
use axum::http::{StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::routing::get;

use crate::{Challenges, Domain};

/// Port 80's answers: Let's Encrypt's HTTP-01 checks from `challenges`,
/// and a permanent move to the same address on `https://domain` for every
/// other request, so a visitor who types the plain address lands on the
/// secure one.
pub fn port_80(domain: &Domain, challenges: Challenges) -> Router {
    let domain = domain.clone();
    Router::new()
        .route("/.well-known/acme-challenge/{token}", get(answer))
        .fallback(move |uri: Uri| async move { moved(&domain, &uri) })
        .with_state(challenges)
}

/// The key authorization for an open check, or 404.
async fn answer(State(challenges): State<Challenges>, Path(token): Path<String>) -> Response {
    match challenges.http_answer(&token) {
        Some(text) => ([(CONTENT_TYPE, "text/plain")], text).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// A 308 to the same path and query over HTTPS, which keeps a form's POST
/// a POST.
fn moved(domain: &Domain, uri: &Uri) -> Response {
    let path = uri.path_and_query().map_or("/", PathAndQuery::as_str);
    let to = format!("https://{domain}{path}");
    (StatusCode::PERMANENT_REDIRECT, [(LOCATION, to)]).into_response()
}
