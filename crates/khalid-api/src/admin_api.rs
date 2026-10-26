//! The Admin API under `/api/admin/v1/`.

use std::sync::Arc;

use axum::extract::{OriginalUri, Path, State};
use axum::{Extension, Json, middleware};
use khalid_core::UserMessage;
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::admin_key::{require, require_admin_key};
use crate::api_version::negotiate_version;
use crate::keys::{KeyStore, Principal, Scope};
use crate::post_store::PostStore;
use crate::problem::{Problem, ProblemType};
use crate::resources::AdminPost;

/// Where the Admin API lives.
pub(crate) const BASE: &str = "/api/admin/v1";

/// The store every post route reads and writes.
type Posts = Arc<dyn PostStore>;

/// The Admin API routes, each declared with `utoipa::path` and registered
/// through `routes!` (RULES.md BL-API-4). Every route needs an
/// integration key or staff token, and checks its own scope.
pub(crate) fn routes(posts: Posts, keys: Arc<dyn KeyStore>) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(read_admin_post))
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

fn missing(path: &str) -> Problem {
    let message = UserMessage::new(
        "No post has this identity.",
        "It may have been deleted, or the identity may be mistyped.",
        "Check the identity, or list the posts to find it.",
    );
    Problem::new(ProblemType::NotFound, &message).with_instance(path)
}
