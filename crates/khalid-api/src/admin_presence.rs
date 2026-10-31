//! Who has a post open in the editor, and its lock,
//! `/api/admin/v1/posts/{id}/presence/` (plan 4.6 and 5.12, KH-EDT-2).

use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::{OriginalUri, Path, State};
use axum::{Extension, Json};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::admin_api::{invalid_body, missing};
use crate::admin_key::require;
use crate::keys::{Principal, Scope};
use crate::presence::{Claim, PostPresence};
use crate::problem::Problem;
use crate::publish_door::PublishDoor;

/// A check-in: what the editor asks for while the post is open.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct CheckIn {
    /// `join` takes the lock only when nobody holds it; `take_over` takes
    /// it from whoever does. `join` when left out.
    #[serde(default)]
    claim: Claim,
}

/// Notes that the caller has the post open and answers who holds its lock.
/// The editor checks in every 20 seconds; someone who stops for a minute
/// lets go. The lock tells the editor who may type: the Admin API still
/// takes any edit and refuses a stale one with 409. Needs `posts:write`.
#[utoipa::path(
    post, path = "/posts/{id}/presence/", tag = "admin posts",
    security(("admin_key" = [])),
    params(("id" = String, Path, description = "The post's identity")),
    request_body = CheckIn,
    responses(
        (status = 200, description = "Who holds the lock, and who else has the post open", body = PostPresence),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks posts:write", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "No post has this identity", body = Problem, content_type = "application/problem+json"),
        (status = 422, description = "The body could not be read", body = Problem, content_type = "application/problem+json"),
    )
)]
pub(crate) async fn check_in(
    State(posts): State<Arc<PublishDoor>>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    body: Result<Json<CheckIn>, JsonRejection>,
) -> Result<Json<PostPresence>, Problem> {
    require(&principal, Scope::PostsWrite)?;
    let Json(body) = body.map_err(|rejection| invalid_body(&rejection.body_text()))?;
    let found = posts.get(&id).await;
    found
        .map_err(|error| Problem::internal(&error))?
        .ok_or_else(|| missing(uri.path()))?;
    let who = principal.id().as_str();
    Ok(Json(posts.check_in(&id, who, body.claim)))
}
