//! The Admin API's tag routes: the names posts are grouped by, renamed or
//! removed in every post at once (plan sections 4.6 and 5.12, KH-API-2).

use std::sync::Arc;

use axum::extract::State;
use axum::{Extension, Json, middleware};
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::admin_entries::Noun;
use crate::admin_key::{AdminDoor, require, require_admin_key};
use crate::api_version::negotiate_version;
use crate::keys::{Principal, Scope};
use crate::problem::Problem;
use crate::resources::AdminTag;
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
/// `posts:write`.
pub(crate) fn routes(tags: Tags, door: AdminDoor) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(list_tags))
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
