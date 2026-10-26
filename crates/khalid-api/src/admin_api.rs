//! The Admin API under `/api/admin/v1/`.

use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::{OriginalUri, Path, State};
use axum::{Extension, Json, middleware};
use chrono::{DateTime, Utc};
use khalid_core::UserMessage;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::admin_key::{require, require_admin_key};
use crate::api_version::negotiate_version;
use crate::keys::{KeyStore, Principal, Scope};
use crate::post_store::{PostStore, UpdatePostError};
use crate::problem::{Problem, ProblemType};
use crate::resources::{AdminPost, PostEdit};

/// Where the Admin API lives.
pub(crate) const BASE: &str = "/api/admin/v1";

/// The store every post route reads and writes.
type Posts = Arc<dyn PostStore>;

/// The Admin API routes, each declared with `utoipa::path` and registered
/// through `routes!` (RULES.md BL-API-4). Every route needs an
/// integration key or staff token, and checks its own scope.
pub(crate) fn routes(posts: Posts, keys: Arc<dyn KeyStore>) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(read_admin_post, edit_admin_post))
        .route_layer(middleware::from_fn_with_state(keys, require_admin_key))
        .route_layer(middleware::from_fn(negotiate_version))
        .with_state(posts)
}

/// Posts as the Admin API wraps them.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct AdminPosts {
    /// The post or posts.
    posts: Vec<AdminPost>,
}

/// Reads one post in any status. Needs `posts:write`.
#[utoipa::path(
    get, path = "/posts/{id}/", tag = "admin posts",
    security(("admin_key" = [])),
    params(("id" = String, Path, description = "The post's identity")),
    responses(
        (status = 200, description = "The post", body = AdminPosts),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks posts:write", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "No post has this identity", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn read_admin_post(
    State(posts): State<Posts>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<AdminPosts>, Problem> {
    require(&principal, Scope::PostsWrite)?;
    let post = posts
        .get(&id)
        .await
        .map_err(|error| Problem::internal(&error))?;
    let post = post.ok_or_else(|| missing(uri.path()))?;
    Ok(Json(AdminPosts { posts: vec![post] }))
}

/// An edit, wrapped the way the Admin API wraps posts: exactly one.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct EditPosts {
    /// The one edit.
    posts: Vec<PostEdit>,
}

/// Edits one post. The edit carries the `updated_at` of the copy it was
/// made on; when the post changed since, the answer is a 409 naming who
/// changed it and when (RULES.md BL-API-7). Needs `posts:write`.
#[utoipa::path(
    put, path = "/posts/{id}/", tag = "admin posts",
    security(("admin_key" = [])),
    params(("id" = String, Path, description = "The post's identity")),
    request_body = EditPosts,
    responses(
        (status = 200, description = "The post as saved", body = AdminPosts),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks posts:write", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "No post has this identity", body = Problem, content_type = "application/problem+json"),
        (status = 409, description = "The post changed after the copy the edit was made on", body = Problem, content_type = "application/problem+json"),
        (status = 422, description = "The body could not be read", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn edit_admin_post(
    State(posts): State<Posts>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    body: Result<Json<EditPosts>, JsonRejection>,
) -> Result<Json<AdminPosts>, Problem> {
    require(&principal, Scope::PostsWrite)?;
    let Json(body) = body.map_err(|rejection| invalid_body(&rejection.body_text()))?;
    let [edit] = <[PostEdit; 1]>::try_from(body.posts)
        .map_err(|_| invalid_body("posts must hold exactly one post"))?;
    match posts.update(&id, edit, principal.id().as_str()).await {
        Ok(post) => Ok(Json(AdminPosts { posts: vec![post] })),
        Err(UpdatePostError::NotFound) => Err(missing(uri.path())),
        Err(UpdatePostError::Stale {
            updated_at,
            updated_by,
        }) => Err(stale(updated_at, updated_by.as_deref())),
        Err(UpdatePostError::Store(error)) => Err(Problem::internal(&error)),
    }
}

fn invalid_body(reason: &str) -> Problem {
    let message = UserMessage::new(
        format!("The request body could not be read: {reason}."),
        "Nothing was changed.",
        "Send JSON shaped like the example in /api/openapi.json, with updated_at.",
    );
    Problem::new(ProblemType::InvalidBody, &message)
}

fn stale(updated_at: DateTime<Utc>, updated_by: Option<&str>) -> Problem {
    let when = updated_at.format("%Y-%m-%d %H:%M UTC");
    let message = UserMessage::new(
        format!(
            "{} changed this post at {when}, after the copy you edited.",
            updated_by.unwrap_or("Someone")
        ),
        "Saving your copy would undo their change.",
        "Fetch the post again, make your change to it, and send its new updated_at.",
    );
    Problem::new(ProblemType::Conflict, &message).with_rule("updated_at")
}

fn missing(path: &str) -> Problem {
    let message = UserMessage::new(
        "No post has this identity.",
        "It may have been deleted, or the identity may be mistyped.",
        "Check the identity, or list the posts to find it.",
    );
    Problem::new(ProblemType::NotFound, &message).with_instance(path)
}
