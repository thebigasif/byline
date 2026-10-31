//! The Admin API's page routes: the site's pages, such as About (plan
//! section 5.12, KH-API-2).

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
use crate::resources::AdminPage;

/// How messages name a page.
const PAGE: Noun = Noun {
    one: "page",
    many: "pages",
};

/// The page routes, under the Admin API's base. Reading needs
/// `posts:write`.
pub(crate) fn routes(pages: Kept<AdminPage>, door: AdminDoor) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(list_pages))
        .route_layer(middleware::from_fn_with_state(door, require_admin_key))
        .route_layer(middleware::from_fn(negotiate_version))
        .with_state(pages)
}

/// Pages as the Admin API wraps them.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct AdminPages {
    /// The page or pages.
    pages: Vec<AdminPage>,
}

/// Lists every page in every language, ordered by language and slug. A
/// site has tens of pages, so the list comes whole. Needs `posts:write`.
#[utoipa::path(
    get, path = "/pages/", tag = "admin pages",
    security(("admin_key" = [])),
    responses(
        (status = 200, description = "Every page", body = AdminPages),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks posts:write", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "This server keeps no pages", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn list_pages(
    State(pages): State<Kept<AdminPage>>,
    Extension(principal): Extension<Principal>,
) -> Result<Json<AdminPages>, Problem> {
    require(&principal, Scope::PostsWrite)?;
    let listed = PAGE.store(pages.as_deref())?.list().await;
    let pages = listed.map_err(|error| Problem::internal(&error))?;
    Ok(Json(AdminPages { pages }))
}
