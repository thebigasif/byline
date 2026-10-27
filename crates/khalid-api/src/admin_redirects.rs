//! The redirect routes of the Admin API under `/api/admin/v1/redirects/`
//! (KH-SEO-8). Every route needs `settings:write`.

use std::sync::Arc;

use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::StatusCode;
use axum::{Extension, Json, middleware};
use chrono::{DateTime, Utc};
use khalid_core::UserMessage;
use khalid_seo::{Matching, RedirectColumns, RedirectEntry, RedirectTable, SeoError};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::admin_key::{require, require_admin_key};
use crate::api_version::negotiate_version;
use crate::keys::{KeyStore, Principal, Scope};
use crate::problem::{Problem, ProblemType};
use crate::redirect_desk::RedirectDesk;
use crate::redirect_store::{Origin, RedirectStoreError, StoredRedirect};

/// The redirect routes, each declared with `utoipa::path` (RULES.md
/// BL-API-4), behind an integration key or staff token.
pub(crate) fn routes(desk: RedirectDesk, keys: Arc<dyn KeyStore>) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(list_redirects, create_redirect))
        .route_layer(middleware::from_fn_with_state(keys, require_admin_key))
        .route_layer(middleware::from_fn(negotiate_version))
        .with_state(desk)
}

/// A stored redirect as the Admin API shows it.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct AdminRedirect {
    /// The identity to remove it by.
    #[schema(example = 12)]
    id: i64,
    /// The path it starts from, or a regular expression over the path.
    #[schema(example = "/blog/old-links/")]
    from: String,
    /// Where a move leads: a path, a web address, or a pattern's template
    /// with `$1` for its first group. Empty for 410 and 451.
    #[schema(example = "/blog/why-internal-links-matter/")]
    to: Option<String>,
    /// 301, 302, 307, 410 or 451.
    #[schema(example = 301)]
    status: u16,
    /// `exact` or `regex`.
    #[serde(rename = "match")]
    #[schema(example = "exact")]
    matching: &'static str,
    /// `manual`, `import` or `slug_change`.
    #[schema(example = "manual")]
    origin: &'static str,
    /// When it was made.
    created_at: DateTime<Utc>,
}

impl From<&StoredRedirect> for AdminRedirect {
    fn from(stored: &StoredRedirect) -> Self {
        let columns = stored.entry.columns();
        Self {
            id: stored.id,
            from: columns.from,
            to: columns.to,
            status: columns.status,
            matching: columns.matching.as_str(),
            origin: stored.origin.as_str(),
            created_at: stored.created_at,
        }
    }
}

/// Redirects as the Admin API wraps them.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct AdminRedirects {
    /// The redirects, oldest first.
    redirects: Vec<AdminRedirect>,
}

/// Lists every stored redirect, oldest first. Aliases in front matter are
/// not listed; they live in their posts. Needs `settings:write`.
#[utoipa::path(
    get, path = "/redirects/", tag = "admin redirects",
    security(("admin_key" = [])),
    responses(
        (status = 200, description = "Every stored redirect", body = AdminRedirects),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks settings:write", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn list_redirects(
    State(desk): State<RedirectDesk>,
    Extension(principal): Extension<Principal>,
) -> Result<Json<AdminRedirects>, Problem> {
    require(&principal, Scope::SettingsWrite)?;
    let stored = desk
        .store
        .list()
        .await
        .map_err(|error| Problem::internal(&error))?;
    let redirects = stored.iter().map(AdminRedirect::from).collect();
    Ok(Json(AdminRedirects { redirects }))
}

/// One new redirect, in the columns [`AdminRedirect`] shows.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct NewRedirect {
    /// The path it starts from, or a regular expression over the path.
    #[schema(example = "/blog/old-links/")]
    from: String,
    /// Where a move leads. Leave it out for 410 and 451.
    #[serde(default)]
    #[schema(example = "/blog/why-internal-links-matter/")]
    to: Option<String>,
    /// 301, 302, 307, 410 or 451.
    #[schema(example = 301)]
    status: u16,
    /// `exact`, the default, or `regex`.
    #[serde(rename = "match", default)]
    #[schema(example = "exact")]
    matching: Option<String>,
}

/// The body of a new redirect: exactly one.
#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct NewRedirects {
    /// The one redirect to store.
    redirects: Vec<NewRedirect>,
}

/// Stores one redirect made by hand. It is checked against every stored
/// redirect, so it can neither take an address that has one nor close a
/// loop. Needs `settings:write`.
#[utoipa::path(
    post, path = "/redirects/", tag = "admin redirects",
    security(("admin_key" = [])),
    request_body = NewRedirects,
    responses(
        (status = 201, description = "The stored redirect", body = AdminRedirects),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks settings:write", body = Problem, content_type = "application/problem+json"),
        (status = 409, description = "The address already has a redirect", body = Problem, content_type = "application/problem+json"),
        (status = 422, description = "The redirect cannot be followed or would close a loop", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn create_redirect(
    State(desk): State<RedirectDesk>,
    Extension(principal): Extension<Principal>,
    body: Result<Json<NewRedirects>, JsonRejection>,
) -> Result<(StatusCode, Json<AdminRedirects>), Problem> {
    require(&principal, Scope::SettingsWrite)?;
    let Json(body) = body.map_err(|rejection| unstorable(&rejection.body_text()))?;
    let [new] = <[NewRedirect; 1]>::try_from(body.redirects)
        .map_err(|_| unstorable("redirects must hold exactly one redirect"))?;
    let matching = new.matching.as_deref().unwrap_or("exact");
    let matching =
        Matching::from_name(matching).ok_or_else(|| unstorable("match must be exact or regex"))?;
    let columns = RedirectColumns::new(&new.from, new.to.as_deref(), new.status, matching);
    let entry = RedirectEntry::from_columns(&columns).map_err(|e| unstorable(&e.to_string()))?;
    let stored = desk.store.list().await.map_err(|e| Problem::internal(&e))?;
    let all = stored.into_iter().map(|s| s.entry).chain([entry.clone()]);
    RedirectTable::from_entries(all.collect()).map_err(|error| refused(&error))?;
    let added = desk
        .store
        .add(vec![entry], Origin::Manual, desk.clock.now())
        .await
        .map_err(|error| match error {
            RedirectStoreError::Taken { from } => taken(&from),
            error => Problem::internal(&error),
        })?;
    let redirects = added.iter().map(AdminRedirect::from).collect();
    Ok((StatusCode::CREATED, Json(AdminRedirects { redirects })))
}

/// The 422 for a redirect that cannot be followed.
fn unstorable(reason: &str) -> Problem {
    let message = UserMessage::new(
        format!("This redirect cannot be stored: {reason}."),
        "A redirect that cannot be followed would send readers nowhere.",
        "Give a from path starting with /, a status of 301, 302, 307, 410 or 451, \
         a to address for a move and none for 410 or 451, and match exact or regex.",
    );
    Problem::new(ProblemType::InvalidBody, &message)
}

/// The answer when the whole table refuses the new redirect.
fn refused(error: &SeoError) -> Problem {
    match error {
        SeoError::RedirectDuplicate { from } => taken(from),
        error => {
            let message = UserMessage::new(
                format!("This redirect cannot be stored: {error}."),
                "Readers and search engines would go round in a loop and never reach a page.",
                "Point it at the final address, or remove the redirect that leads back.",
            );
            Problem::new(ProblemType::InvalidBody, &message)
        }
    }
}

/// The 409 for an address that already has a redirect.
fn taken(from: &str) -> Problem {
    let message = UserMessage::new(
        format!("The address {from} already has a redirect."),
        "One address can lead to only one place.",
        "Remove the redirect it has first, then add the new one.",
    );
    Problem::new(ProblemType::Conflict, &message)
}
