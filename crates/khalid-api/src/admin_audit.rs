//! The audit log through the Admin API, `GET /api/admin/v1/audit/`
//! (KH-EDT-6).

use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use axum::{Extension, Json, middleware};
use chrono::{DateTime, Utc};
use khalid_core::UserMessage;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::admin_key::{AdminDoor, require, require_admin_key};
use crate::api_version::negotiate_version;
use crate::audit_desk::AuditDesk;
use crate::audit_log::Logged;
use crate::keys::{Principal, Scope};
use crate::problem::{Problem, ProblemType};

/// How many changes a page holds when none is asked for.
const DEFAULT_LIMIT: u32 = 50;

/// The most changes one page holds.
const MAX_LIMIT: u32 = 100;

/// The audit route, behind an integration key or staff token. Without a
/// desk it lists nothing.
pub(crate) fn routes(desk: Option<AuditDesk>, door: AdminDoor) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(list_changes))
        .route_layer(middleware::from_fn_with_state(door, require_admin_key))
        .route_layer(middleware::from_fn(negotiate_version))
        .with_state(desk)
}

/// Which page of the log to list.
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
#[serde(deny_unknown_fields)]
pub(crate) struct ChangesQuery {
    /// Only changes older than this one, by its `id`: the `next` of the
    /// page before. Left out, the list starts at the newest.
    before: Option<i64>,
    /// From 1 to 100; 50 when left out.
    limit: Option<u32>,
}

/// One change, as the Admin API shows it.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct AdminChange {
    /// Its place in the log: a later change has a larger number.
    id: i64,
    /// When it was made.
    at: DateTime<Utc>,
    /// The identity of the key or staff account that made it.
    by: String,
    /// `added`, `changed` or `removed`.
    action: String,
    /// The route it went through.
    #[schema(example = "/api/admin/v1/posts/{id}/")]
    route: String,
    /// The address it was made at.
    #[schema(example = "/api/admin/v1/posts/en.internal-links/")]
    path: String,
}

/// One page of the audit log.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct AdminChanges {
    /// The changes, newest first.
    changes: Vec<AdminChange>,
    /// The `before` that lists the next page, or null on the last page.
    next: Option<i64>,
}

/// Lists who changed what through the Admin API and when, newest first:
/// every write that succeeded, to content, settings or staff accounts.
/// Needs `admin:all`.
#[utoipa::path(
    get, path = "/audit/", tag = "admin audit",
    security(("admin_key" = [])),
    params(ChangesQuery),
    responses(
        (status = 200, description = "One page of the log", body = AdminChanges),
        (status = 400, description = "The limit is not from 1 to 100, or before is not a number", body = Problem, content_type = "application/problem+json"),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks admin:all", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn list_changes(
    State(desk): State<Option<AuditDesk>>,
    Extension(principal): Extension<Principal>,
    query: Result<Query<ChangesQuery>, QueryRejection>,
) -> Result<Json<AdminChanges>, Problem> {
    require(&principal, Scope::AdminAll)?;
    let Query(query) = query.map_err(|_| bad_page())?;
    let limit = query.limit.unwrap_or(DEFAULT_LIMIT);
    if !(1..=MAX_LIMIT).contains(&limit) {
        return Err(bad_page());
    }
    let changes = match desk {
        Some(desk) => desk.log.changes(query.before, limit).await,
        None => Ok(Vec::new()),
    };
    let changes = changes.map_err(|error| Problem::internal(&error))?;
    let full = changes.len() == usize::try_from(limit).unwrap_or(usize::MAX);
    let next = changes.last().filter(|_| full).map(|last| last.id);
    Ok(Json(AdminChanges {
        changes: changes.into_iter().map(shown).collect(),
        next,
    }))
}

fn shown(logged: Logged) -> AdminChange {
    let Logged { id, change } = logged;
    AdminChange {
        id,
        at: change.at,
        by: change.by,
        action: change.action.as_str().to_owned(),
        route: change.route,
        path: change.path,
    }
}

/// The 400 for a page that cannot be read.
fn bad_page() -> Problem {
    let message = UserMessage::new(
        "The page of the audit log asked for cannot be read: limit must be from 1 to 100, and before a change's id.",
        "A page that large would be slow to read and to send.",
        "Ask for limit=50, and pass the next of the page before as before.",
    );
    Problem::new(ProblemType::InvalidQuery, &message).with_rule("limit")
}
