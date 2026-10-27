//! The Admin API's comment routes: the moderation queue and approve
//! (plan Phase 5, task 5.4).

use std::sync::Arc;

use axum::extract::rejection::QueryRejection;
use axum::extract::{OriginalUri, Path, Query, State};

use axum::{Extension, Json, middleware};
use khalid_core::UserMessage;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::admin_key::{require, require_admin_key};
use crate::api_version::negotiate_version;
use crate::comment_moderation::{CommentModeration, ModerationError};
use crate::keys::{KeyStore, Principal, Scope};
use crate::problem::{Problem, ProblemType};
use crate::resources::{AdminComment, CommentStatus};

/// The moderation every comment route calls.
type Moderation = Arc<dyn CommentModeration>;

/// How many comments a list sends when it is not told.
const DEFAULT_LIMIT: u32 = 50;

/// The most comments one list sends.
const MAX_LIMIT: u32 = 100;

/// The comment routes, under the Admin API's base. Each needs an
/// integration key or staff token with `posts:write`: whoever may change
/// the posts moderates the comments on them.
pub(crate) fn routes(comments: Moderation, keys: Arc<dyn KeyStore>) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(list_comments))
        .routes(routes!(approve_comment))
        .route_layer(middleware::from_fn_with_state(keys, require_admin_key))
        .route_layer(middleware::from_fn(negotiate_version))
        .with_state(comments)
}

/// Comments as the Admin API wraps them.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct AdminComments {
    /// The comment or comments.
    comments: Vec<AdminComment>,
}

/// What a comment list asks for.
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
#[serde(deny_unknown_fields)]
pub(crate) struct CommentQuery {
    /// Only comments with this status, such as `pending` for the queue.
    status: Option<CommentStatus>,
    /// How many to send, newest first: 50 when not given, at most 100.
    limit: Option<u32>,
}

/// Lists the newest comments, filtered by status. Needs `posts:write`.
#[utoipa::path(
    get, path = "/comments/", tag = "admin comments",
    security(("admin_key" = [])),
    params(CommentQuery),
    responses(
        (status = 200, description = "The comments, newest first", body = AdminComments),
        (status = 400, description = "The status or limit is not one the API knows", body = Problem, content_type = "application/problem+json"),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks posts:write", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn list_comments(
    State(comments): State<Moderation>,
    Extension(principal): Extension<Principal>,
    query: Result<Query<CommentQuery>, QueryRejection>,
) -> Result<Json<AdminComments>, Problem> {
    require(&principal, Scope::PostsWrite)?;
    let Query(query) = query.map_err(|_| bad_query())?;
    let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let listed = comments.list(query.status, limit).await;
    let comments = listed.map_err(|error| problem(error, ""))?;
    Ok(Json(AdminComments { comments }))
}

/// Shows a comment under its post. Needs `posts:write`.
#[utoipa::path(
    post, path = "/comments/{id}/approve/", tag = "admin comments",
    security(("admin_key" = [])),
    params(("id" = i64, Path, description = "The comment's identity")),
    responses(
        (status = 200, description = "The comment, approved", body = AdminComments),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks posts:write", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "No comment has this identity", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn approve_comment(
    State(comments): State<Moderation>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<AdminComments>, Problem> {
    require(&principal, Scope::PostsWrite)?;
    let id = id.parse().map_err(|_| missing(uri.path()))?;
    let approved = comments.approve(id).await;
    let comment = approved.map_err(|error| problem(error, uri.path()))?;
    Ok(Json(AdminComments {
        comments: vec![comment],
    }))
}

/// The problem a moderation error at `path` answers with.
fn problem(error: ModerationError, path: &str) -> Problem {
    match error {
        ModerationError::NotFound => missing(path),
        ModerationError::Invalid { field, reason } => invalid(&reason, Some(field)),
        error => Problem::internal(&error),
    }
}

fn missing(path: &str) -> Problem {
    let message = UserMessage::new(
        "No comment has this identity.",
        "It may have been deleted with the comment it answered, or the identity may be mistyped.",
        "List the comments to find it.",
    );
    Problem::new(ProblemType::NotFound, &message).with_instance(path)
}

fn invalid(reason: &str, field: Option<&str>) -> Problem {
    let message = UserMessage::new(
        format!("The reply could not be kept: {reason}."),
        "Nothing was changed.",
        "Send one reply with an author_name and a text, shaped like the example in /api/openapi.json.",
    );
    let problem = Problem::new(ProblemType::InvalidBody, &message);
    match field {
        Some(field) => problem.with_rule(field),
        None => problem,
    }
}

fn bad_query() -> Problem {
    let message = UserMessage::new(
        "The comment list was asked for in a way the API does not know.",
        "status takes pending, approved, spam or trash, and limit takes a whole number.",
        "Fix the query, or leave it out to list the newest 50 comments.",
    );
    Problem::new(ProblemType::InvalidQuery, &message).with_rule("status")
}
