//! The Admin API's earlier versions of a post,
//! `GET /api/admin/v1/posts/{id}/versions/` (plan 4.6, KH-EDT-2).

use std::sync::Arc;

use axum::extract::{OriginalUri, Path, State};
use axum::{Extension, Json};
use khalid_core::UserMessage;
use serde::Serialize;
use utoipa::ToSchema;

use crate::admin_api::missing;
use crate::admin_key::require;
use crate::keys::{Principal, Scope};
use crate::problem::{Problem, ProblemType};
use crate::publish_door::PublishDoor;
use crate::version_store::PostVersion;

/// Earlier versions, wrapped the way the Admin API wraps resources.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PostVersions {
    /// The newest first.
    versions: Vec<PostVersion>,
}

/// Lists the title and text each edit of the post replaced, the newest
/// first, up to the 100 newest. An edit that changed neither keeps none.
/// Bringing one back is an ordinary edit with its title and text, which
/// keeps the text it replaces in turn. Needs `posts:write`.
#[utoipa::path(
    get, path = "/posts/{id}/versions/", tag = "admin posts",
    security(("admin_key" = [])),
    params(("id" = String, Path, description = "The post's identity")),
    responses(
        (status = 200, description = "The earlier versions, the newest first", body = PostVersions),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks posts:write", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "No post has this identity, or this server keeps no versions", body = Problem, content_type = "application/problem+json"),
    )
)]
pub(crate) async fn list_post_versions(
    State(posts): State<Arc<PublishDoor>>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<PostVersions>, Problem> {
    require(&principal, Scope::PostsWrite)?;
    let found = posts.get(&id).await;
    found
        .map_err(|error| Problem::internal(&error))?
        .ok_or_else(|| missing(uri.path()))?;
    let versions = posts.versions(&id).await;
    let versions = versions.map_err(|error| Problem::internal(&error))?;
    let versions = versions.ok_or_else(|| none_kept(uri.path()))?;
    Ok(Json(PostVersions { versions }))
}

/// The 404 from a server that keeps no versions, such as one reading
/// posts without a database.
fn none_kept(path: &str) -> Problem {
    let message = UserMessage::new(
        "This server keeps no earlier versions.",
        "Earlier versions are kept in the site's database, which this server does not open.",
        "Ask for them from the running khalid serve.",
    );
    Problem::new(ProblemType::NotFound, &message).with_instance(path)
}
