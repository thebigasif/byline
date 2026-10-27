//! The key every Admin API request carries, and the scope check each
//! route makes.

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::header::{AUTHORIZATION, COOKIE, WWW_AUTHENTICATE};
use axum::http::{HeaderMap, HeaderValue};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use khalid_core::UserMessage;

use crate::keys::{
    KeyError, KeyKind, KeyStore, KeyStoreError, KeyToken, Principal, Scope, authenticate,
};
use crate::problem::{Problem, ProblemType};
use crate::session_check::{CSRF_HEADER, SessionCheck};

/// The `Authorization` scheme for integration keys and staff tokens.
const SCHEME: &str = "Khalid";

/// What lets a request into the Admin API: the stored keys, and staff
/// sessions when the site has accounts.
#[derive(Clone)]
pub(crate) struct AdminDoor {
    pub(crate) keys: Arc<dyn KeyStore>,
    pub(crate) sessions: Option<Arc<dyn SessionCheck>>,
}

/// Lets a request through only with a stored integration key or staff
/// token in `Authorization: Khalid <key>`, or, with no `Authorization`
/// header, a staff session cookie with its CSRF token (RULES.md BL-SEC-9),
/// and hands the routes the [`Principal`] it belongs to.
pub(crate) async fn require_admin_key(
    State(door): State<AdminDoor>,
    mut request: Request,
    next: Next,
) -> Response {
    let principal = if request.headers().contains_key(AUTHORIZATION) {
        by_key(door.keys.as_ref(), request.headers()).await
    } else {
        by_session(door.sessions.as_deref(), request.headers()).await
    };
    match principal {
        Ok(Some(principal)) => {
            request.extensions_mut().insert(principal);
            next.run(request).await
        }
        Ok(None) => refused(),
        Err(source) => Problem::internal(&source).into_response(),
    }
}

async fn by_key(
    keys: &dyn KeyStore,
    headers: &HeaderMap,
) -> Result<Option<Principal>, KeyStoreError> {
    let Some(text) = presented(headers) else {
        return Ok(None);
    };
    let Ok(kind @ (KeyKind::Integration | KeyKind::Staff)) =
        text.parse::<KeyToken>().map(|token| token.kind())
    else {
        return Ok(None);
    };
    match authenticate(keys, &text, kind).await {
        Ok(principal) => Ok(Some(principal)),
        Err(KeyError::Unavailable { source }) => Err(source),
        Err(_) => Ok(None),
    }
}

async fn by_session(
    sessions: Option<&dyn SessionCheck>,
    headers: &HeaderMap,
) -> Result<Option<Principal>, KeyStoreError> {
    let header = |name: &str| headers.get(name).and_then(|value| value.to_str().ok());
    match (sessions, header(COOKIE.as_str())) {
        (Some(sessions), Some(cookies)) => sessions.check(cookies, header(CSRF_HEADER)).await,
        _ => Ok(None),
    }
}

/// The key from `Authorization: Khalid <key>`. Another scheme is no key.
fn presented(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(AUTHORIZATION)?.to_str().ok()?;
    value
        .strip_prefix(SCHEME)?
        .strip_prefix(' ')
        .map(str::to_owned)
}

fn refused() -> Response {
    let message = UserMessage::new(
        "This request has no valid integration key or staff token.",
        "The Admin API can change the site, so it answers only known keys.",
        "Send Authorization: Khalid followed by a key made with khalid keys create.",
    );
    let mut response = Problem::new(ProblemType::Unauthorized, &message).into_response();
    response
        .headers_mut()
        .insert(WWW_AUTHENTICATE, HeaderValue::from_static(SCHEME));
    response
}

/// Refuses with a 403 problem naming `scope` when `principal` lacks it.
pub(crate) fn require(principal: &Principal, scope: Scope) -> Result<(), Problem> {
    if principal.access().allows(scope) {
        return Ok(());
    }
    let message = UserMessage::new(
        format!("This key does not have the {} scope.", scope.as_str()),
        "Each key may do only what it was made for, so a leaked key does less harm.",
        format!(
            "Use a key with {}, or make one with that scope.",
            scope.as_str()
        ),
    );
    Err(Problem::new(ProblemType::Forbidden, &message).with_rule(scope.as_str()))
}
