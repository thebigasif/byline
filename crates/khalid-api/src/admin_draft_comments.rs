//! The Admin API's routes for comments on drafts, under
//! `/api/admin/v1/posts/{id}/draft-comments/` (KH-EDT-4). Every route needs
//! `posts:write`.

use std::num::NonZeroU32;
use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::{OriginalUri, Path, State};
use axum::http::StatusCode;
use axum::{Extension, Json, middleware};
use khalid_core::UserMessage;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::admin_key::{AdminDoor, require, require_admin_key};
use crate::api_version::negotiate_version;
use crate::draft_comments::{DraftComment, DraftCommentText, DraftComments, NewDraftComment};
use crate::keys::{Principal, Scope};
use crate::problem::{Problem, ProblemType};
use crate::publish_door::PublishDoor;
use crate::resources::AdminPost;

/// The posts the comments are on, and the comments, or `None` for a site
/// that keeps none.
pub(crate) type Desk = (Arc<PublishDoor>, Option<DraftComments>);

/// The routes, behind an integration key or staff token.
pub(crate) fn routes(
    door: Arc<PublishDoor>,
    comments: Option<DraftComments>,
    keys: AdminDoor,
) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(
            crate::admin_draft_threads::list_draft_comments,
            add_draft_comment
        ))
        .route_layer(middleware::from_fn_with_state(keys, require_admin_key))
        .route_layer(middleware::from_fn(negotiate_version))
        .with_state((door, comments))
}

/// Comments as the Admin API wraps them.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct DraftCommentList {
    /// The comments, oldest first.
    pub(crate) draft_comments: Vec<DraftComment>,
}

/// A new comment as a client sends it.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct DraftCommentBody {
    /// What it says, up to 5,000 characters.
    #[schema(value_type = String, example = "Can we name the tool here?")]
    text: DraftCommentText,
    /// The block of the body it is about, counted from 1. Left out, it is
    /// about the post as a whole. A reply is about its thread's block.
    #[schema(value_type = Option<u32>, minimum = 1, example = 3)]
    block: Option<NonZeroU32>,
    /// The first comment of the thread it answers. Threads are one level
    /// deep.
    parent: Option<i64>,
}

/// One new comment, wrapped the way the Admin API wraps resources.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct NewDraftComments {
    /// Exactly one comment.
    draft_comments: Vec<DraftCommentBody>,
}

/// The post `id`, or the 404 for `path`.
pub(crate) async fn post(door: &PublishDoor, id: &str, path: &str) -> Result<AdminPost, Problem> {
    let post = door.get(id).await.map_err(|e| Problem::internal(&e))?;
    post.ok_or_else(|| crate::admin_api::missing(path))
}

/// The comments, or the 422 for a site that keeps none.
pub(crate) fn kept(comments: Option<DraftComments>) -> Result<DraftComments, Problem> {
    comments.ok_or_else(|| {
        let message = UserMessage::new(
            "This site keeps no comments on drafts.",
            "Comments on drafts are kept by the site's server, in its database.",
            "Send the request to the running khalid serve.",
        );
        Problem::new(ProblemType::InvalidBody, &message)
    })
}

/// The 422 for a body that could not be read.
pub(crate) fn unreadable(reason: &str) -> Problem {
    let message = UserMessage::new(
        format!("The comment could not be read: {reason}."),
        "A comment needs text, and may name a block from 1 or the thread it answers.",
        "Send {\"draft_comments\": [{\"text\": \"...\"}]}.",
    );
    Problem::new(ProblemType::InvalidBody, &message)
}

/// Adds a comment to a post, about one block or the whole post, or as a
/// reply in a thread, and tells the people in the thread and the site's
/// editors. Needs `posts:write`.
#[utoipa::path(
    post, path = "/posts/{id}/draft-comments/", tag = "admin posts",
    security(("admin_key" = [])),
    params(("id" = String, Path, description = "The post's identity")),
    request_body = NewDraftComments,
    responses(
        (status = 201, description = "The comment as saved", body = DraftCommentList),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks posts:write", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "No post has this identity", body = Problem, content_type = "application/problem+json"),
        (status = 422, description = "The body could not be read, the parent is not a thread on this post, or the site keeps no comments on drafts", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn add_draft_comment(
    State((door, comments)): State<Desk>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    body: Result<Json<NewDraftComments>, JsonRejection>,
) -> Result<(StatusCode, Json<DraftCommentList>), Problem> {
    require(&principal, Scope::PostsWrite)?;
    let Json(body) = body.map_err(|rejection| unreadable(&rejection.body_text()))?;
    let [body] = <[DraftCommentBody; 1]>::try_from(body.draft_comments)
        .map_err(|_| unreadable("draft_comments must hold exactly one comment"))?;
    let post = post(&door, &id, uri.path()).await?;
    let comments = kept(comments)?;
    let earlier = comments
        .store()
        .list(&id)
        .await
        .map_err(|e| Problem::internal(&e))?;
    let mut new = NewDraftComment::new(&id, principal.id().as_str(), body.text);
    new.block = body.block.map(NonZeroU32::get);
    if let Some(parent) = body.parent {
        let thread = earlier
            .iter()
            .find(|c| c.id == parent && c.parent.is_none());
        let thread = thread.ok_or_else(no_thread)?;
        (new.parent, new.block) = (Some(parent), thread.block);
    }
    let saved = comments
        .store()
        .add(new)
        .await
        .map_err(|e| Problem::internal(&e))?;
    comments.notice().added(&post, &saved, &earlier).await;
    Ok((
        StatusCode::CREATED,
        Json(DraftCommentList {
            draft_comments: vec![saved],
        }),
    ))
}

/// The 422 for a parent that does not begin a thread on the post.
fn no_thread() -> Problem {
    let message = UserMessage::new(
        "The parent is not the first comment of a thread on this post.",
        "Threads are one level deep, so a reply answers a thread's first comment.",
        "Send the id of the thread's first comment as parent, or leave parent out.",
    );
    Problem::new(ProblemType::InvalidBody, &message).with_rule("parent")
}
