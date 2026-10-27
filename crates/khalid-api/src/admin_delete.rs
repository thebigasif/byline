//! The Admin API's post delete, `DELETE /api/admin/v1/posts/{id}/`.

use std::sync::Arc;

use axum::Extension;
use axum::extract::rejection::QueryRejection;
use axum::extract::{OriginalUri, Path, Query, State};
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use khalid_core::UserMessage;
use serde::Deserialize;
use utoipa::IntoParams;

use crate::admin_api::{missing, stale};
use crate::admin_key::{require, require_publish};
use crate::keys::{Principal, Scope};
use crate::post_store::DeletePostError;
use crate::problem::{Problem, ProblemType};
use crate::publish_door::PublishDoor;
use crate::resources::PostStatus;

/// What a delete names: the copy of the post it was made on.
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeleteQuery {
    /// The post's `updated_at` as last read, such as `2026-10-20T08:30:00Z`.
    #[param(value_type = String, required = true)]
    updated_at: Option<DateTime<Utc>>,
}

/// Deletes one post in any status, with its file and its schedule. The
/// delete names the `updated_at` of the copy it was made on; when the post
/// changed since, the answer is a 409 naming who changed it and when
/// (RULES.md BL-API-7). Needs `posts:write`, and `posts:publish` for a
/// post that is live or scheduled.
#[utoipa::path(
    delete, path = "/posts/{id}/", tag = "admin posts",
    security(("admin_key" = [])),
    params(("id" = String, Path, description = "The post's identity"), DeleteQuery),
    responses(
        (status = 204, description = "The post is gone"),
        (status = 400, description = "updated_at is missing or is not a time", body = Problem, content_type = "application/problem+json"),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks posts:write, or posts:publish for a live or scheduled post", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "No post has this identity", body = Problem, content_type = "application/problem+json"),
        (status = 409, description = "The post changed after the copy the delete was made on", body = Problem, content_type = "application/problem+json"),
    )
)]
pub(crate) async fn delete_admin_post(
    State(posts): State<Arc<PublishDoor>>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    query: Result<Query<DeleteQuery>, QueryRejection>,
) -> Result<StatusCode, Problem> {
    require(&principal, Scope::PostsWrite)?;
    let updated_at = query
        .ok()
        .and_then(|Query(query)| query.updated_at)
        .ok_or_else(no_copy)?;
    if !principal.access().allows(Scope::PostsPublish) {
        let post = posts.get(&id).await;
        let post = post.map_err(|error| Problem::internal(&error))?;
        if post.is_some_and(|post| post.status() != PostStatus::Draft) {
            require_publish(&principal)?;
        }
    }
    match posts.delete(&id, updated_at).await {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(DeletePostError::NotFound) => Err(missing(uri.path())),
        Err(DeletePostError::Stale {
            updated_at,
            updated_by,
        }) => Err(stale(updated_at, updated_by.as_deref())),
        Err(error) => Err(Problem::internal(&error)),
    }
}

/// The 400 for a delete that does not say which copy it was made on.
fn no_copy() -> Problem {
    let message = UserMessage::new(
        "This delete does not say which copy of the post it was made on.",
        "Without it, a post someone changed a moment ago could be deleted unseen.",
        "Add updated_at with the time the post last changed, such as ?updated_at=2026-10-20T08:30:00Z.",
    );
    Problem::new(ProblemType::InvalidQuery, &message).with_rule("updated_at")
}
