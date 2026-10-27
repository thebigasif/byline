//! Signing an Admin API request in by a staff session cookie, beside
//! integration keys and staff tokens.

use std::fmt;

use utoipa_axum::router::OpenApiRouter;

use crate::content_store::BoxFuture;
use crate::keys::{KeyStoreError, Principal};

/// The header a request signed in by cookie carries with the session's
/// CSRF token (RULES.md BL-SEC-9).
pub const CSRF_HEADER: &str = "x-khalid-csrf";

/// Checks a staff session cookie. `khalid-auth` implements it; the Admin
/// API asks it only when a request carries no `Authorization` header.
pub trait SessionCheck: Send + Sync {
    /// Who the session in `cookies`, the request's `Cookie` header, signs
    /// in, when `csrf`, its [`CSRF_HEADER`], is that session's token.
    /// `None` for no live session, or a missing or wrong token: a cookie
    /// is sent by the browser on its own, so without the token it proves
    /// nothing.
    ///
    /// # Errors
    ///
    /// Returns [`KeyStoreError`] when the sessions cannot be read.
    fn check<'a>(
        &'a self,
        cookies: &'a str,
        csrf: Option<&'a str>,
    ) -> BoxFuture<'a, Result<Option<Principal>, KeyStoreError>>;
}

/// The staff routes another crate adds to the Admin API: `open` routes
/// need no key, such as signing in; `signed_in` routes get the same key
/// check as every other Admin API route, and find the [`Principal`] in the
/// request's extensions.
pub struct StaffRoutes {
    /// Routes anyone may call.
    pub open: OpenApiRouter,
    /// Routes that need a key, a staff token or a session.
    pub signed_in: OpenApiRouter,
}

impl fmt::Debug for StaffRoutes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StaffRoutes").finish_non_exhaustive()
    }
}
