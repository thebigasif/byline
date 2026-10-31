//! The Admin API's tag routes: the names posts are grouped by, renamed or
//! removed in every post at once (plan sections 4.6 and 5.12, KH-API-2).

use std::sync::Arc;

use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{OriginalUri, Path, Query, State};
use axum::http::StatusCode;
use axum::{Extension, Json, middleware};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::admin_delete::DeleteQuery;
use crate::admin_entries::Noun;
use crate::admin_key::{AdminDoor, require, require_admin_key};
use crate::api_version::negotiate_version;
use crate::keys::{Principal, Scope};
use crate::problem::Problem;
use crate::resources::{AdminTag, TagEdit};
use crate::tag_store::TagStore;

/// How messages name a tag.
const TAG: Noun = Noun {
    one: "tag",
    many: "tags",
    example: "en.internal-links",
};

/// The site's tags, when the site keeps them.
type Tags = Option<Arc<dyn TagStore>>;

/// The tag routes, under the Admin API's base. Reading needs
/// `posts:write`; a change rewrites live posts, so it needs `posts:publish`.
pub(crate) fn routes(tags: Tags, door: AdminDoor) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(list_tags))
        .routes(routes!(rename_tag, remove_tag))
        .route_layer(middleware::from_fn_with_state(door, require_admin_key))
        .route_layer(middleware::from_fn(negotiate_version))
        .with_state(tags)
}

/// Tags as the Admin API wraps them.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct AdminTags {
    /// The tag or tags.
    tags: Vec<AdminTag>,
}

/// Lists every tag the site's posts carry, drafts included, in every
/// language, with how many posts have each. Needs `posts:write`.
#[utoipa::path(
    get, path = "/tags/", tag = "admin tags",
    security(("admin_key" = [])),
    responses(
        (status = 200, description = "Every tag", body = AdminTags),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks posts:write", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "This server keeps no tags", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn list_tags(
    State(tags): State<Tags>,
    Extension(principal): Extension<Principal>,
) -> Result<Json<AdminTags>, Problem> {
    require(&principal, Scope::PostsWrite)?;
    let listed = TAG.store(tags.as_deref())?.list().await;
    let tags = listed.map_err(|error| Problem::internal(&error))?;
    Ok(Json(AdminTags { tags }))
}

/// A rename, wrapped the way the Admin API wraps tags: exactly one.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct TagEdits {
    /// The one rename.
    tags: Vec<TagEdit>,
}

/// Renames a tag in every post that has it, drafts included; a name
/// another tag has merges the two. The rename carries the tag's
/// `updated_at`; when one of its posts changed since, the answer is a 409
/// and no post changes (RULES.md BL-API-7). Needs `posts:publish`.
#[utoipa::path(
    put, path = "/tags/{id}/", tag = "admin tags",
    security(("admin_key" = [])),
    params(("id" = String, Path, description = "The language and slug, such as en.internal-links")),
    request_body = TagEdits,
    responses(
        (status = 200, description = "The tag under its new name", body = AdminTags),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks posts:publish", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "No post has this tag, or this server keeps no tags", body = Problem, content_type = "application/problem+json"),
        (status = 409, description = "A post with the tag changed after the copy the rename was made on", body = Problem, content_type = "application/problem+json"),
        (status = 422, description = "The body could not be read, or the name is empty or has no letter or digit", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn rename_tag(
    State(tags): State<Tags>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    body: Result<Json<TagEdits>, JsonRejection>,
) -> Result<Json<AdminTags>, Problem> {
    require(&principal, Scope::PostsPublish)?;
    let edit = TAG.one(TAG.body(body)?.tags)?;
    TAG.filled("name", edit.name())?;
    let renamed = TAG.store(tags.as_deref())?.rename(&id, edit).await;
    let tag = renamed.map_err(|error| TAG.problem(error, uri.path()))?;
    Ok(Json(AdminTags { tags: vec![tag] }))
}

/// Removes a tag from every post that has it, drafts included. The
/// request names the tag's `updated_at`; when one of its posts changed
/// since, the answer is a 409 and no post changes. Needs `posts:publish`.
#[utoipa::path(
    delete, path = "/tags/{id}/", tag = "admin tags",
    security(("admin_key" = [])),
    params(("id" = String, Path, description = "The language and slug, such as en.internal-links"), DeleteQuery),
    responses(
        (status = 204, description = "No post has the tag any more"),
        (status = 400, description = "updated_at is missing or is not a time", body = Problem, content_type = "application/problem+json"),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks posts:publish", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "No post has this tag, or this server keeps no tags", body = Problem, content_type = "application/problem+json"),
        (status = 409, description = "A post with the tag changed after the copy the request was made on", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn remove_tag(
    State(tags): State<Tags>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    query: Result<Query<DeleteQuery>, QueryRejection>,
) -> Result<StatusCode, Problem> {
    require(&principal, Scope::PostsPublish)?;
    let seen = query.ok().and_then(|Query(query)| query.updated_at);
    let seen = seen.ok_or_else(|| TAG.no_copy())?;
    let removed = TAG.store(tags.as_deref())?.remove(&id, seen).await;
    removed.map_err(|error| TAG.problem(error, uri.path()))?;
    Ok(StatusCode::NO_CONTENT)
}
