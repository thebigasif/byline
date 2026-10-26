//! The key every Admin API request carries, and the scope check each
//! route makes.

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::HeaderValue;
use axum::http::header::{AUTHORIZATION, WWW_AUTHENTICATE};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use khalid_core::UserMessage;

use crate::keys::{KeyError, KeyKind, KeyStore, KeyToken, Principal, Scope, authenticate};
use crate::problem::{Problem, ProblemType};

/// The `Authorization` scheme for integration keys and staff tokens.
const SCHEME: &str = "Khalid";

/// Lets a request through only with a stored integration key or staff
/// token in `Authorization: Khalid <key>`, and hands the routes the
/// [`Principal`] it belongs to.
pub(crate) async fn require_admin_key(
    State(keys): State<Arc<dyn KeyStore>>,
    mut request: Request,
    next: Next,
) -> Response {
    let Some(text) = presented(&request) else {
        return refused();
    };
    let Ok(kind @ (KeyKind::Integration | KeyKind::Staff)) =
        text.parse::<KeyToken>().map(|token| token.kind())
    else {
        return refused();
    };
    match authenticate(keys.as_ref(), &text, kind).await {
        Ok(principal) => {
            request.extensions_mut().insert(principal);
            next.run(request).await
        }
        Err(KeyError::Unavailable { source }) => Problem::internal(&source).into_response(),
        Err(_) => refused(),
    }
}

/// The key from `Authorization: Khalid <key>`. Another scheme is no key.
fn presented(request: &Request) -> Option<String> {
    let value = request.headers().get(AUTHORIZATION)?.to_str().ok()?;
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
