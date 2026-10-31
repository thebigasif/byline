//! The trash through the Admin API (KH-EDT-6): list what was deleted,
//! restore it, or delete it for good.

use std::sync::Arc;

use axum::extract::{OriginalUri, Path, State};
use axum::http::StatusCode;
use axum::{Extension, Json, middleware};
use chrono::{DateTime, Utc};
use khalid_core::UserMessage;
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::admin_api::AdminPosts;
use crate::admin_key::{AdminDoor, require, require_admin_key, require_publish};
use crate::api_version::negotiate_version;
use crate::keys::{Principal, Scope};
use crate::post_trash::{PostTrash, TrashError, Trashed};
use crate::problem::{Problem, ProblemType};
use crate::resources::PostStatus;

/// The trash the routes read and write, or `None` when the site keeps
/// none: the list is empty and every key is unknown.
type Trash = Option<Arc<dyn PostTrash>>;

/// The trash routes, behind an integration key or staff token.
pub(crate) fn routes(trash: Trash, door: AdminDoor) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(list_trash))
        .routes(routes!(purge_trashed))
        .routes(routes!(restore_trashed))
        .route_layer(middleware::from_fn_with_state(door, require_admin_key))
        .route_layer(middleware::from_fn(negotiate_version))
        .with_state(trash)
}

/// A deleted post, as the Admin API shows it.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct TrashedPost {
    /// Names this copy in the trash, for restoring or deleting it.
    #[schema(example = "20261031T093000Z-en.internal-links")]
    key: String,
    /// The post's identity when it was deleted.
    #[schema(example = "en.internal-links")]
    id: String,
    /// Its title.
    title: String,
    /// Its status when it was deleted.
    status: PostStatus,
    /// When it was deleted.
    trashed_at: DateTime<Utc>,
}

/// The trash as the Admin API wraps it.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct TrashList {
    /// The deleted posts, the most recently deleted first.
    trash: Vec<TrashedPost>,
}

/// Lists the deleted posts, the most recently deleted first. Needs
/// `posts:write`.
#[utoipa::path(
    get, path = "/trash/", tag = "admin trash",
    security(("admin_key" = [])),
    responses(
        (status = 200, description = "The deleted posts", body = TrashList),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks posts:write", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn list_trash(
    State(trash): State<Trash>,
    Extension(principal): Extension<Principal>,
) -> Result<Json<TrashList>, Problem> {
    require(&principal, Scope::PostsWrite)?;
    let listed = trashed(trash.as_deref()).await?;
    let trash = listed.into_iter().map(|trashed| TrashedPost {
        key: trashed.key,
        id: trashed.id,
        title: trashed.title,
        status: trashed.status,
        trashed_at: trashed.trashed_at,
    });
    Ok(Json(TrashList {
        trash: trash.collect(),
    }))
}

/// Puts a deleted post back where it was, in the status it had, and
/// answers it as it is now. Needs `posts:write`, and `posts:publish` for a
/// post that was live or scheduled.
#[utoipa::path(
    post, path = "/trash/{key}/restore/", tag = "admin trash",
    security(("admin_key" = [])),
    params(("key" = String, Path, description = "The deleted post's key in the trash")),
    responses(
        (status = 200, description = "The restored post", body = AdminPosts),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks posts:write, or posts:publish for a post that was live or scheduled", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "Nothing in the trash has this key", body = Problem, content_type = "application/problem+json"),
        (status = 409, description = "Another post has the address the restored one would take", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn restore_trashed(
    State(trash): State<Trash>,
    Extension(principal): Extension<Principal>,
    Path(key): Path<String>,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<AdminPosts>, Problem> {
    require(&principal, Scope::PostsWrite)?;
    let trash = trash.as_deref().ok_or_else(|| not_in_trash(uri.path()))?;
    let listed = trashed(Some(trash)).await?;
    let found = listed.iter().find(|trashed| trashed.key == key);
    let found = found.ok_or_else(|| not_in_trash(uri.path()))?;
    if found.status != PostStatus::Draft {
        require_publish(&principal)?;
    }
    match trash.restore(&key).await {
        Ok(post) => Ok(Json(AdminPosts { posts: vec![post] })),
        Err(error) => Err(refused(&error, uri.path())),
    }
}

/// Deletes a post in the trash for good. Needs `posts:publish`.
#[utoipa::path(
    delete, path = "/trash/{key}/", tag = "admin trash",
    security(("admin_key" = [])),
    params(("key" = String, Path, description = "The deleted post's key in the trash")),
    responses(
        (status = 204, description = "The post is gone for good"),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks posts:publish", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "Nothing in the trash has this key", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn purge_trashed(
    State(trash): State<Trash>,
    Extension(principal): Extension<Principal>,
    Path(key): Path<String>,
    OriginalUri(uri): OriginalUri,
) -> Result<StatusCode, Problem> {
    require_publish(&principal)?;
    let trash = trash.as_deref().ok_or_else(|| not_in_trash(uri.path()))?;
    match trash.purge(&key).await {
        Ok(()) => Ok(StatusCode::NO_CONTENT),
        Err(error) => Err(refused(&error, uri.path())),
    }
}

/// Every post in `trash`, or none when the site keeps no trash.
async fn trashed(trash: Option<&dyn PostTrash>) -> Result<Vec<Trashed>, Problem> {
    match trash {
        Some(trash) => trash.trashed().await.map_err(|e| Problem::internal(&e)),
        None => Ok(Vec::new()),
    }
}

/// The problem for a restore or delete the trash refused.
fn refused(error: &TrashError, path: &str) -> Problem {
    match error {
        TrashError::NotFound => not_in_trash(path),
        TrashError::Taken { id } => {
            let message = UserMessage::new(
                format!("Post {id} has the address this post had."),
                "Two posts cannot share one address, so restoring it would hide one of them.",
                "Change the slug of the post that has the address now, then restore this one.",
            );
            Problem::new(ProblemType::Conflict, &message).with_instance(path)
        }
        TrashError::Store(error) => Problem::internal(error),
    }
}

/// The 404 for a key nothing in the trash has.
fn not_in_trash(path: &str) -> Problem {
    let message = UserMessage::new(
        "Nothing in the trash has this key.",
        "It may have been restored or deleted for good already.",
        "List the trash to find the post you want.",
    );
    Problem::new(ProblemType::NotFound, &message).with_instance(path)
}
