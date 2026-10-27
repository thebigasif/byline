//! The 404 log through the Admin API, each address with the page nearest
//! to it (KH-SEO-8). Needs `settings:write`.

use std::sync::Arc;

use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use axum::{Extension, Json, middleware};
use chrono::{DateTime, Utc};
use khalid_core::UserMessage;
use khalid_seo::{UrlPath, nearest_page};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::admin_key::{require, require_admin_key};
use crate::api_version::negotiate_version;
use crate::keys::{KeyStore, Principal, Scope};
use crate::problem::{Problem, ProblemType};
use crate::redirect_desk::RedirectDesk;

/// How many logged addresses a list holds when none is asked for.
const DEFAULT_LIMIT: u32 = 100;

/// The most logged addresses one list holds.
const MAX_LIMIT: u32 = 1_000;

/// The 404 routes, behind an integration key or staff token.
pub(crate) fn routes(desk: RedirectDesk, keys: Arc<dyn KeyStore>) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(list_misses))
        .route_layer(middleware::from_fn_with_state(keys, require_admin_key))
        .route_layer(middleware::from_fn(negotiate_version))
        .with_state(desk)
}

/// How many logged addresses to list.
#[derive(Debug, Deserialize, IntoParams)]
pub(crate) struct MissesQuery {
    /// From 1 to 1,000; 100 when left out.
    limit: Option<u32>,
}

/// An address that answered 404, as the Admin API shows it.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct AdminMiss {
    /// The address as requested.
    #[schema(example = "/blog/internal-links/")]
    path: String,
    /// How many requests it answered.
    hits: u64,
    /// The first request.
    first_seen: DateTime<Utc>,
    /// The latest request.
    last_seen: DateTime<Utc>,
    /// The page "redirect this" would send it to, or null when no page is
    /// near enough to guess.
    #[schema(example = "/blog/why-internal-links-matter/")]
    nearest: Option<String>,
}

/// The 404 log as the Admin API wraps it.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct AdminMisses {
    /// The addresses, most requested first.
    misses: Vec<AdminMiss>,
}

/// Lists the addresses that answered 404, most requested first, each with
/// the nearest page. Needs `settings:write`.
#[utoipa::path(
    get, path = "/not-found/", tag = "admin redirects",
    security(("admin_key" = [])),
    params(MissesQuery),
    responses(
        (status = 200, description = "The logged addresses", body = AdminMisses),
        (status = 400, description = "The limit is not from 1 to 1,000", body = Problem, content_type = "application/problem+json"),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks settings:write", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn list_misses(
    State(desk): State<RedirectDesk>,
    Extension(principal): Extension<Principal>,
    query: Result<Query<MissesQuery>, QueryRejection>,
) -> Result<Json<AdminMisses>, Problem> {
    require(&principal, Scope::SettingsWrite)?;
    let limit = query.ok().map(|Query(q)| q.limit.unwrap_or(DEFAULT_LIMIT));
    let limit = limit
        .filter(|n| (1..=MAX_LIMIT).contains(n))
        .ok_or_else(|| {
            let message = UserMessage::new(
                "The limit is not a number from 1 to 1,000.",
                "A list that large would be slow to read and to send.",
                "Ask for limit=100, or leave it out.",
            );
            Problem::new(ProblemType::InvalidQuery, &message).with_rule("limit")
        })?;
    let misses = desk
        .log
        .misses(limit)
        .await
        .map_err(|e| Problem::internal(&e))?;
    let pages = desk
        .pages
        .addresses()
        .await
        .map_err(|e| Problem::internal(&e))?;
    let misses = misses.into_iter().map(|miss| {
        let path = UrlPath::try_new(&miss.path).ok();
        let nearest = path.and_then(|path| nearest_page(&path, &pages).map(ToString::to_string));
        AdminMiss {
            path: miss.path,
            hits: miss.hits,
            first_seen: miss.first_seen,
            last_seen: miss.last_seen,
            nearest,
        }
    });
    Ok(Json(AdminMisses {
        misses: misses.collect(),
    }))
}
