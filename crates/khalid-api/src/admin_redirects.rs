//! The redirect routes of the Admin API under `/api/admin/v1/redirects/`
//! (KH-SEO-8). Every route needs `settings:write`.

use std::sync::Arc;

use axum::extract::State;
use axum::{Extension, Json, middleware};
use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::admin_key::{require, require_admin_key};
use crate::api_version::negotiate_version;
use crate::keys::{KeyStore, Principal, Scope};
use crate::problem::Problem;
use crate::redirect_desk::RedirectDesk;
use crate::redirect_store::StoredRedirect;

/// The redirect routes, each declared with `utoipa::path` (RULES.md
/// BL-API-4), behind an integration key or staff token.
pub(crate) fn routes(desk: RedirectDesk, keys: Arc<dyn KeyStore>) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(list_redirects))
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
