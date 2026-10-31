//! The trash through the Admin API (KH-EDT-6): what was deleted, listed
//! so it can be restored or deleted for good.

use std::sync::Arc;

use axum::extract::State;
use axum::{Extension, Json, middleware};
use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::admin_key::{AdminDoor, require, require_admin_key};
use crate::api_version::negotiate_version;
use crate::keys::{Principal, Scope};
use crate::post_trash::{PostTrash, Trashed};
use crate::problem::Problem;
use crate::resources::PostStatus;

/// The trash the routes read and write, or `None` when the site keeps
/// none: the list is empty and every key is unknown.
type Trash = Option<Arc<dyn PostTrash>>;

/// The trash routes, behind an integration key or staff token.
pub(crate) fn routes(trash: Trash, door: AdminDoor) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(list_trash))
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

/// Every post in `trash`, or none when the site keeps no trash.
async fn trashed(trash: Option<&dyn PostTrash>) -> Result<Vec<Trashed>, Problem> {
    match trash {
        Some(trash) => trash.trashed().await.map_err(|e| Problem::internal(&e)),
        None => Ok(Vec::new()),
    }
}
