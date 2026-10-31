//! The Admin API's author routes: the people bylines, author pages and the
//! author schema name (plan sections 4.4 and 5.12, KH-API-2).

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
use crate::resources::AdminAuthor;

/// How messages name a author.
const AUTHOR: Noun = Noun {
    one: "author",
    many: "authors",
    example: "en.asif",
};

/// The author routes, under the Admin API's base. Reading needs
/// `posts:write`; an author shows on live posts once saved, so writing
/// needs `posts:publish`.
pub(crate) fn routes(authors: Kept<AdminAuthor>, door: AdminDoor) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(list_authors))
        .route_layer(middleware::from_fn_with_state(door, require_admin_key))
        .route_layer(middleware::from_fn(negotiate_version))
        .with_state(authors)
}

/// Authors as the Admin API wraps them.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct AdminAuthors {
    /// The author or authors.
    authors: Vec<AdminAuthor>,
}

/// Lists every author in every language. A site has tens, so the list
/// comes whole. Needs `posts:write`.
#[utoipa::path(
    get, path = "/authors/", tag = "admin authors",
    security(("admin_key" = [])),
    responses(
        (status = 200, description = "Every author", body = AdminAuthors),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks posts:write", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "This server keeps no authors", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn list_authors(
    State(authors): State<Kept<AdminAuthor>>,
    Extension(principal): Extension<Principal>,
) -> Result<Json<AdminAuthors>, Problem> {
    require(&principal, Scope::PostsWrite)?;
    let listed = AUTHOR.store(authors.as_deref())?.list().await;
    let authors = listed.map_err(|error| Problem::internal(&error))?;
    Ok(Json(AdminAuthors { authors }))
}
