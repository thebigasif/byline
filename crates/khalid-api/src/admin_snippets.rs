//! The Admin API's snippet routes: blocks saved once, such as an affiliate
//! disclosure, that every post placing them shows (plan sections 4.6 and
//! 5.12, KH-API-2).

use axum::extract::rejection::JsonRejection;
use axum::extract::{OriginalUri, Path, State};
use axum::http::header::LOCATION;
use axum::http::{HeaderName, StatusCode};
use axum::{Extension, Json, middleware};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::admin_api::BASE;
use crate::admin_entries::{Kept, Noun};
use crate::admin_key::{AdminDoor, require, require_admin_key};
use crate::api_version::negotiate_version;
use crate::entry_store::Entry;
use crate::keys::{Principal, Scope};
use crate::problem::Problem;
use crate::resources::{AdminSnippet, NewSnippet};

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
        .routes(routes!(list_snippets, create_snippet))
        .routes(routes!(read_snippet))
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

/// A new snippet, wrapped the way the Admin API wraps snippets: exactly one.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct NewSnippets {
    /// The one new snippet.
    snippets: Vec<NewSnippet>,
}

/// Creates a snippet in the language it names or the site's default.
/// Needs `posts:publish`.
#[utoipa::path(
    post, path = "/snippets/", tag = "admin snippets",
    security(("admin_key" = [])),
    request_body = NewSnippets,
    responses(
        (status = 201, description = "The new snippet", body = AdminSnippets),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks posts:publish", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "This server keeps no snippets", body = Problem, content_type = "application/problem+json"),
        (status = 409, description = "Another snippet in that language has the slug", body = Problem, content_type = "application/problem+json"),
        (status = 422, description = "The body could not be read, the title is empty, or the site has no such language", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn create_snippet(
    State(snippets): State<Kept<AdminSnippet>>,
    Extension(principal): Extension<Principal>,
    body: Result<Json<NewSnippets>, JsonRejection>,
) -> Result<(StatusCode, [(HeaderName, String); 1], Json<AdminSnippets>), Problem> {
    require(&principal, Scope::PostsPublish)?;
    let new = SNIPPET.one(SNIPPET.body(body)?.snippets)?;
    SNIPPET.filled("title", new.title())?;
    let made = SNIPPET.store(snippets.as_deref())?.create(new).await;
    let snippet = made.map_err(|error| SNIPPET.problem(error, ""))?;
    let location = format!("{BASE}/snippets/{}/", snippet.id());
    let snippets = vec![snippet];
    Ok((
        StatusCode::CREATED,
        [(LOCATION, location)],
        Json(AdminSnippets { snippets }),
    ))
}
/// Reads one snippet. Needs `posts:write`.
#[utoipa::path(
    get, path = "/snippets/{id}/", tag = "admin snippets",
    security(("admin_key" = [])),
    params(("id" = String, Path, description = "The language and slug, such as en.affiliate-disclosure")),
    responses(
        (status = 200, description = "The snippet", body = AdminSnippets),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks posts:write", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "No snippet has this identity, or this server keeps no snippets", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn read_snippet(
    State(snippets): State<Kept<AdminSnippet>>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<AdminSnippets>, Problem> {
    require(&principal, Scope::PostsWrite)?;
    let found = SNIPPET.store(snippets.as_deref())?.get(&id).await;
    let snippet = found.map_err(|error| Problem::internal(&error))?;
    let snippet = snippet.ok_or_else(|| SNIPPET.missing(uri.path()))?;
    Ok(Json(AdminSnippets {
        snippets: vec![snippet],
    }))
}
