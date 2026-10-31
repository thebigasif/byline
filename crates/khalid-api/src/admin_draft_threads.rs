//! The Admin API's routes that read comments on drafts as threads and
//! resolve them, under `/api/admin/v1/posts/{id}/draft-comments/`
//! (KH-EDT-4). Every route needs `posts:write`.

use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::{OriginalUri, Path, State};
use axum::http::StatusCode;
use axum::{Extension, Json, middleware};
use serde::Deserialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::admin_draft_comments::{Desk, DraftCommentList, kept, post, unreadable};
use crate::admin_key::{AdminDoor, require, require_admin_key};
use crate::api_version::negotiate_version;
use crate::draft_comments::{DraftComment, DraftComments, ThreadState};
use crate::keys::{Principal, Scope};
use crate::problem::Problem;
use crate::publish_door::PublishDoor;

/// The routes, behind an integration key or staff token. The list shares
/// its path with the add route, so it is registered there.
pub(crate) fn routes(
    door: Arc<PublishDoor>,
    comments: Option<DraftComments>,
    keys: AdminDoor,
) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(resolve_draft_comment))
        .route_layer(middleware::from_fn_with_state(keys, require_admin_key))
        .route_layer(middleware::from_fn(negotiate_version))
        .with_state((door, comments))
}

/// Whether a thread is resolved.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ResolveBody {
    /// `true` once the thread is dealt with, `false` to open it again.
    resolved: bool,
}

/// A thread's new state, wrapped the way the Admin API wraps resources.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ResolveThreads {
    /// Exactly one change.
    draft_comments: Vec<ResolveBody>,
}

/// Lists every comment on a post, oldest first, each reply after the
/// comment it answers. Needs `posts:write`.
#[utoipa::path(
    get, path = "/posts/{id}/draft-comments/", tag = "admin posts",
    security(("admin_key" = [])),
    params(("id" = String, Path, description = "The post's identity")),
    responses(
        (status = 200, description = "The comments, oldest first, replies after their thread's first comment", body = DraftCommentList),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks posts:write", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "No post has this identity", body = Problem, content_type = "application/problem+json"),
    )
)]
pub(crate) async fn list_draft_comments(
    State((door, comments)): State<Desk>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<DraftCommentList>, Problem> {
    require(&principal, Scope::PostsWrite)?;
    post(&door, &id, uri.path()).await?;
    let all = match comments {
        Some(comments) => comments.store().list(&id).await,
        None => Ok(Vec::new()),
    };
    let all = all.map_err(|e| Problem::internal(&e))?;
    let draft_comments = threaded(all);
    Ok(Json(DraftCommentList { draft_comments }))
}

/// `all` with each reply moved after the comment it answers.
fn threaded(all: Vec<DraftComment>) -> Vec<DraftComment> {
    let (firsts, replies): (Vec<_>, Vec<_>) = all.into_iter().partition(|c| c.parent.is_none());
    let mut ordered = Vec::with_capacity(firsts.len() + replies.len());
    for first in firsts {
        let id = first.id;
        ordered.push(first);
        ordered.extend(replies.iter().filter(|r| r.parent == Some(id)).cloned());
    }
    ordered
}

/// Marks a thread resolved once it is dealt with, or open again. Send the
/// id of the thread's first comment. Needs `posts:write`.
#[utoipa::path(
    put, path = "/posts/{id}/draft-comments/{comment}/", tag = "admin posts",
    security(("admin_key" = [])),
    params(
        ("id" = String, Path, description = "The post's identity"),
        ("comment" = i64, Path, description = "The thread's first comment"),
    ),
    request_body = ResolveThreads,
    responses(
        (status = 204, description = "The thread is resolved or open as asked"),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks posts:write", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "No post has this identity, or no thread on it begins with this comment", body = Problem, content_type = "application/problem+json"),
        (status = 422, description = "The body could not be read, or the site keeps no comments on drafts", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn resolve_draft_comment(
    State((door, comments)): State<Desk>,
    Extension(principal): Extension<Principal>,
    Path((id, comment)): Path<(String, i64)>,
    OriginalUri(uri): OriginalUri,
    body: Result<Json<ResolveThreads>, JsonRejection>,
) -> Result<StatusCode, Problem> {
    require(&principal, Scope::PostsWrite)?;
    let Json(body) = body.map_err(|rejection| unreadable(&rejection.body_text()))?;
    let [body] = <[ResolveBody; 1]>::try_from(body.draft_comments)
        .map_err(|_| unreadable("draft_comments must hold exactly one change"))?;
    post(&door, &id, uri.path()).await?;
    let comments = kept(comments)?;
    let state = ThreadState::from(body.resolved);
    let found = comments.store().resolve(&id, comment, state).await;
    if found.map_err(|e| Problem::internal(&e))? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(crate::admin_api::missing(uri.path()))
    }
}
