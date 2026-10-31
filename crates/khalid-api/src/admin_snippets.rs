//! The Admin API's snippet routes: blocks saved once, such as an affiliate
//! disclosure, that every post placing them shows (plan sections 4.6 and
//! 5.12, KH-API-2).

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
use crate::resources::AdminSnippet;

/// How messages name a snippet.
const SNIPPET: Noun = Noun {
    one: "snippet",
    many: "snippets",
    example: "en.affiliate-disclosure",
};

/// The snippet routes, under the Admin API's base. Reading needs
/// `posts:write`; a snippet shows in live posts once saved, so writing
/// needs `posts:publish`.
pub(crate) fn routes(snippets: Kept<AdminSnippet>, door: AdminDoor) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(list_snippets))
        .route_layer(middleware::from_fn_with_state(door, require_admin_key))
        .route_layer(middleware::from_fn(negotiate_version))
        .with_state(snippets)
}

/// Snippets as the Admin API wraps them.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct AdminSnippets {
    /// The snippet or snippets.
    snippets: Vec<AdminSnippet>,
}

/// Lists every snippet in every language. A site has tens, so the list
/// comes whole. Needs `posts:write`.
#[utoipa::path(
    get, path = "/snippets/", tag = "admin snippets",
    security(("admin_key" = [])),
    responses(
        (status = 200, description = "Every snippet", body = AdminSnippets),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks posts:write", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "This server keeps no snippets", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn list_snippets(
    State(snippets): State<Kept<AdminSnippet>>,
    Extension(principal): Extension<Principal>,
) -> Result<Json<AdminSnippets>, Problem> {
    require(&principal, Scope::PostsWrite)?;
    let listed = SNIPPET.store(snippets.as_deref())?.list().await;
    let snippets = listed.map_err(|error| Problem::internal(&error))?;
    Ok(Json(AdminSnippets { snippets }))
}
