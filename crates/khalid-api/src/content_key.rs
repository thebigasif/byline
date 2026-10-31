//! The content key every Content API request carries.

use std::sync::Arc;

use axum::extract::{Query, Request, State};
use axum::http::HeaderValue;
use axum::http::header::{AUTHORIZATION, WWW_AUTHENTICATE};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use khalid_core::UserMessage;
use serde::Deserialize;

use crate::api_limits::{self, Api, ApiLimits};
use crate::keys::{KeyError, KeyKind, KeyStore, authenticate};
use crate::problem::{Problem, ProblemType};

/// The `Authorization` scheme for content keys.
pub(crate) const SCHEME: &str = "Khalid-Content";

/// What lets a request into the Content API: the stored keys and the
/// request limits.
#[derive(Clone)]
pub(crate) struct ContentDoor {
    pub(crate) keys: Arc<dyn KeyStore>,
    pub(crate) limits: Arc<ApiLimits>,
}

/// Lets a request through only with a stored content key, sent as
/// `Authorization: Khalid-Content <key>` or as `?key=<key>`. A busy key,
/// or an address that sent too many wrong keys, gets a 429 (BL-SEC-5).
pub(crate) async fn require_content_key(
    State(door): State<ContentDoor>,
    request: Request,
    next: Next,
) -> Response {
    let address = api_limits::client(&request);
    if let Some(wait) = address.and_then(|address| door.limits.paused(address)) {
        return api_limits::paused(wait);
    }
    let Some(text) = presented(&request) else {
        return refused();
    };
    match authenticate(door.keys.as_ref(), &text, KeyKind::Content).await {
        Ok(principal) => match door.limits.admit(principal.id(), Api::Content) {
            Ok(()) => next.run(request).await,
            Err(wait) => api_limits::busy(wait),
        },
        Err(KeyError::Unavailable { source }) => Problem::internal(&source).into_response(),
        Err(_) => {
            if let Some(address) = address {
                door.limits.refused(address);
            }
            refused()
        }
    }
}

/// The one query parameter this layer reads.
#[derive(Deserialize)]
struct KeyParameter {
    key: Option<String>,
}

/// The key from the `Authorization` header, or else from `?key=`. A
/// header with another scheme counts as no key.
fn presented(request: &Request) -> Option<String> {
    if let Some(value) = request.headers().get(AUTHORIZATION) {
        let value = value.to_str().ok()?;
        return value
            .strip_prefix(SCHEME)?
            .strip_prefix(' ')
            .map(str::to_owned);
    }
    let Query(parameter) = Query::<KeyParameter>::try_from_uri(request.uri()).ok()?;
    parameter.key
}

/// A 401 problem that says how to send the key, with the scheme in
/// `WWW-Authenticate`.
fn refused() -> Response {
    let message = UserMessage::new(
        "This request has no valid content key.",
        "The Content API answers only requests that carry the site's content key.",
        "Send it as Authorization: Khalid-Content followed by the key, or add ?key= to the address.",
    );
    let mut response = Problem::new(ProblemType::Unauthorized, &message).into_response();
    let scheme = HeaderValue::from_static(SCHEME);
    response.headers_mut().insert(WWW_AUTHENTICATE, scheme);
    response
}
