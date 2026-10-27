//! The 404 log through the Admin API, and "redirect this": one step from
//! an address that found nothing to a permanent move (KH-SEO-8). Both
//! routes need `settings:write`.

use std::sync::Arc;

use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::{Extension, Json, middleware};
use chrono::{DateTime, Utc};
use khalid_core::UserMessage;
use khalid_seo::{Destination, Move, Redirect, RedirectEntry, RedirectRule, UrlPath, nearest_page};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::admin_key::{require, require_admin_key};
use crate::admin_redirects::{AdminRedirects, store_checked};
use crate::api_version::negotiate_version;
use crate::keys::{KeyStore, Principal, Scope};
use crate::problem::{Problem, ProblemType};
use crate::redirect_desk::RedirectDesk;
use crate::redirect_store::Origin;

/// How many logged addresses a list holds when none is asked for.
const DEFAULT_LIMIT: u32 = 100;

/// The most logged addresses one list holds.
const MAX_LIMIT: u32 = 1_000;

/// The 404 routes, behind an integration key or staff token.
pub(crate) fn routes(desk: RedirectDesk, keys: Arc<dyn KeyStore>) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(list_misses))
        .routes(routes!(redirect_miss))
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

/// The address to redirect, and where to.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct RedirectMiss {
    /// The address that answered 404.
    #[schema(example = "/blog/internal-links/")]
    path: String,
    /// The page to send it to. Leave it out for the nearest page.
    #[serde(default)]
    #[schema(example = "/blog/why-internal-links-matter/")]
    to: Option<String>,
}

/// "Redirect this": stores a permanent move from an address that answered
/// 404 to the page given, or to the nearest page, and drops the address
/// from the log. Needs `settings:write`.
#[utoipa::path(
    post, path = "/not-found/redirect/", tag = "admin redirects",
    security(("admin_key" = [])),
    request_body = RedirectMiss,
    responses(
        (status = 201, description = "The stored redirect", body = AdminRedirects),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks settings:write", body = Problem, content_type = "application/problem+json"),
        (status = 409, description = "The address already has a redirect", body = Problem, content_type = "application/problem+json"),
        (status = 422, description = "The address or page is not a clean path, or no page is near", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn redirect_miss(
    State(desk): State<RedirectDesk>,
    Extension(principal): Extension<Principal>,
    body: Result<Json<RedirectMiss>, JsonRejection>,
) -> Result<(StatusCode, Json<AdminRedirects>), Problem> {
    require(&principal, Scope::SettingsWrite)?;
    let Json(body) = body.map_err(|rejection| unreadable(&rejection.body_text()))?;
    let from = UrlPath::try_new(&body.path).map_err(|e| unreadable(&e.to_string()))?;
    let to = if let Some(to) = body.to.as_deref() {
        UrlPath::try_new(to).map_err(|e| unreadable(&e.to_string()))?
    } else {
        let pages = desk.pages.addresses().await;
        let pages = pages.map_err(|e| Problem::internal(&e))?;
        nearest_page(&from, &pages).cloned().ok_or_else(none_near)?
    };
    let moved = Redirect::Move {
        kind: Move::Permanent,
        to: Destination::Path(to),
    };
    let entry = RedirectEntry::Exact(RedirectRule::new(from, moved));
    let added = store_checked(&desk, vec![entry], Origin::Manual).await?;
    desk.log
        .forget(&body.path)
        .await
        .map_err(|e| Problem::internal(&e))?;
    Ok((StatusCode::CREATED, Json(added)))
}

/// The 422 for a body that cannot be read.
fn unreadable(reason: &str) -> Problem {
    let message = UserMessage::new(
        format!("This address cannot be redirected: {reason}."),
        "Nothing was stored.",
        "Send path as it was requested, starting with /, and to as a page's path if you choose one.",
    );
    Problem::new(ProblemType::InvalidBody, &message)
}

/// The 422 when no page is near enough to guess.
fn none_near() -> Problem {
    let message = UserMessage::new(
        "No page is near this address, so Khalid will not guess where it should go.",
        "A guess that is wrong sends readers and search engines to the wrong page.",
        "Send to with the page it should go to, or leave it as a 404.",
    );
    Problem::new(ProblemType::InvalidBody, &message)
}
