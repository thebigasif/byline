//! The key every Admin API request carries, and the scope check each
//! route makes.

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::header::{AUTHORIZATION, COOKIE, WWW_AUTHENTICATE};
use axum::http::{HeaderMap, HeaderValue};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use khalid_core::UserMessage;

use crate::api_limits::{self, Api, ApiLimits};
use crate::audit_desk::AuditDesk;
use crate::keys::{
    Access, KeyError, KeyKind, KeyStore, KeyStoreError, KeyToken, Principal, Scope, authenticate,
};
use crate::problem::{Problem, ProblemType};
use crate::session_check::{CSRF_HEADER, SessionCheck};

/// The `Authorization` scheme for integration keys and staff tokens.
const SCHEME: &str = "Khalid";

/// What lets a request into the Admin API: the stored keys, staff
/// sessions when the site has accounts, and the request limits.
#[derive(Clone)]
pub(crate) struct AdminDoor {
    pub(crate) keys: Arc<dyn KeyStore>,
    pub(crate) sessions: Option<Arc<dyn SessionCheck>>,
    pub(crate) limits: Arc<ApiLimits>,
    /// Where each write that succeeds is logged, when the site keeps an
    /// audit log (KH-EDT-6).
    pub(crate) audit: Option<AuditDesk>,
}

/// Lets a request through only with a stored integration key or staff
/// token in `Authorization: Khalid <key>`, or, with no `Authorization`
/// header, a staff session cookie with its CSRF token (RULES.md BL-SEC-9),
/// and hands the routes the [`Principal`] it belongs to. A busy key, or an
/// address that sent too many wrong keys, gets a 429 (BL-SEC-5). A write
/// that succeeds is logged in the audit log, so every route is covered.
pub(crate) async fn require_admin_key(
    State(door): State<AdminDoor>,
    mut request: Request,
    next: Next,
) -> Response {
    let address = api_limits::client(&request);
    if let Some(wait) = address.and_then(|address| door.limits.paused(address)) {
        return api_limits::paused(wait);
    }
    let with_key = request.headers().contains_key(AUTHORIZATION);
    let principal = if with_key {
        by_key(door.keys.as_ref(), request.headers()).await
    } else {
        by_session(door.sessions.as_deref(), request.headers()).await
    };
    match principal {
        Ok(Some(principal)) => {
            if let Err(wait) = door.limits.admit(principal.id(), Api::Admin) {
                return api_limits::busy(wait);
            }
            let pending = door.audit.as_ref().and_then(|_| {
                let extensions = request.extensions();
                let (route, uri) = (extensions.get(), extensions.get());
                AuditDesk::pending(request.method(), route, uri, &principal)
            });
            request.extensions_mut().insert(principal);
            let response = next.run(request).await;
            if let (Some(audit), Some(pending)) = (&door.audit, pending)
                && response.status().is_success()
            {
                audit.keep(pending).await;
            }
            response
        }
        Ok(None) => {
            if let Some(address) = address.filter(|_| with_key) {
                door.limits.refused(address);
            }
            refused()
        }
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

/// Refuses with a 403 naming `posts:publish` when `principal` may not
/// publish. A staff member reads it about their role; a key, about its
/// scope.
pub(crate) fn require_publish(principal: &Principal) -> Result<(), Problem> {
    let scope = Scope::PostsPublish;
    let Access::Staff(role) = principal.access() else {
        return require(principal, scope);
    };
    if principal.access().allows(scope) {
        return Ok(());
    }
    let message = UserMessage::new(
        format!("A {} cannot publish posts.", role.as_str()),
        "Only editors, admins and the owner publish, so an editor checks each post before readers see it.",
        "Save your changes as a draft and ask an editor to publish it.",
    );
    Err(Problem::new(ProblemType::Forbidden, &message).with_rule(scope.as_str()))
}
