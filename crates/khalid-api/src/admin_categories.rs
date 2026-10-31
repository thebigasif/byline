//! The Admin API's category routes: the groups posts belong to, each named
//! in every language (plan sections 4.4 and 5.12, KH-API-2).

use axum::extract::State;
use axum::{Extension, Json, middleware};
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::admin_entries::{Kept, Noun};
use crate::admin_key::{AdminDoor, require, require_admin_key};
use crate::api_version::negotiate_version;
use crate::keys::{Principal, Scope};
use crate::problem::Problem;
use crate::resources::AdminCategory;

/// How messages name a category.
const CATEGORY: Noun = Noun {
    one: "category",
    many: "categories",
    example: "on-page-seo",
};

/// The category routes, under the Admin API's base. Reading needs
/// `posts:write`; a category's name shows on live posts and list pages
/// once saved, so writing needs `posts:publish`.
pub(crate) fn routes(categories: Kept<AdminCategory>, door: AdminDoor) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(list_categories))
        .route_layer(middleware::from_fn_with_state(door, require_admin_key))
        .route_layer(middleware::from_fn(negotiate_version))
        .with_state(categories)
}

/// Categories as the Admin API wraps them.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct AdminCategories {
    /// The category or categories.
    categories: Vec<AdminCategory>,
}

/// Lists every category with its names and how many posts are in it. A
/// site has tens, so the list comes whole. Needs `posts:write`.
#[utoipa::path(
    get, path = "/categories/", tag = "admin categories",
    security(("admin_key" = [])),
    responses(
        (status = 200, description = "Every category", body = AdminCategories),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks posts:write", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "This server keeps no categories", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn list_categories(
    State(categories): State<Kept<AdminCategory>>,
    Extension(principal): Extension<Principal>,
) -> Result<Json<AdminCategories>, Problem> {
    require(&principal, Scope::PostsWrite)?;
    let listed = CATEGORY.store(categories.as_deref())?.list().await;
    let categories = listed.map_err(|error| Problem::internal(&error))?;
    Ok(Json(AdminCategories { categories }))
}
