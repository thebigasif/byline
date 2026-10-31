//! `GET /api/admin/v1/posts/{id}/check/`: how a post reads to search
//! engines before it goes live, for the editor's SEO panel (G54).

use std::sync::Arc;

use axum::extract::{OriginalUri, Path, State};
use axum::{Extension, Json, middleware};
use khalid_core::UserMessage;
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::admin_api::missing;
use crate::admin_key::{AdminDoor, require, require_admin_key};
use crate::api_version::negotiate_version;
use crate::keys::{Principal, Scope};
use crate::post_check::{PostCheck, PostChecks};
use crate::problem::{Problem, ProblemType};

/// The site's post checks, or `None` for a server that makes none.
pub(crate) type Checks = Option<Arc<dyn PostChecks>>;

/// The check route, declared with `utoipa::path` (RULES.md BL-API-4),
/// behind an integration key or staff token.
pub(crate) fn routes(checks: Checks, door: AdminDoor) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(check_post))
        .route_layer(middleware::from_fn_with_state(door, require_admin_key))
        .route_layer(middleware::from_fn(negotiate_version))
        .with_state(checks)
}

/// Checks as the Admin API wraps them.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct AdminChecks {
    /// The one post's check.
    checks: Vec<PostCheck>,
}

/// Checks one post as it would go live now, saving nothing: every lint
/// finding with the score out of 100, the search result its page would
/// make, and the JSON-LD the page would carry. A draft is checked as if
/// published today. Needs `posts:write`.
#[utoipa::path(
    get, path = "/posts/{id}/check/", tag = "admin posts",
    security(("admin_key" = [])),
    params(("id" = String, Path, description = "The post's identity")),
    responses(
        (status = 200, description = "The post's check", body = AdminChecks),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks posts:write", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "No post has this identity, or this server checks no posts", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn check_post(
    State(checks): State<Checks>,
    Extension(principal): Extension<Principal>,
    OriginalUri(uri): OriginalUri,
    Path(id): Path<String>,
) -> Result<Json<AdminChecks>, Problem> {
    require(&principal, Scope::PostsWrite)?;
    let checks = checks.ok_or_else(no_checks)?;
    let check = checks
        .check(&id)
        .await
        .map_err(|e| Problem::internal(&e))?
        .ok_or_else(|| missing(uri.path()))?;
    Ok(Json(AdminChecks {
        checks: vec![check],
    }))
}

fn no_checks() -> Problem {
    let message = UserMessage::new(
        "This server does not check posts.",
        "Posts are checked by the site's own server, which reads its settings and its other posts.",
        "Send the request to the running khalid serve.",
    );
    Problem::new(ProblemType::NotFound, &message)
}
