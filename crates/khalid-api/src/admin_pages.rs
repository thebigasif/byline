//! The Admin API's page routes: the site's pages, such as About (plan
//! section 5.12, KH-API-2).

use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{OriginalUri, Path, Query, State};
use axum::http::header::LOCATION;
use axum::http::{HeaderName, StatusCode};
use axum::{Extension, Json, middleware};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::admin_api::BASE;
use crate::admin_delete::DeleteQuery;
use crate::admin_entries::{Kept, Noun};
use crate::admin_key::{AdminDoor, require, require_admin_key};
use crate::api_version::negotiate_version;
use crate::entry_store::Entry;
use crate::keys::{Principal, Scope};
use crate::problem::Problem;
use crate::resources::{AdminPage, NewPage, PageEdit};

/// How messages name a page.
const PAGE: Noun = Noun {
    one: "page",
    many: "pages",
};

/// The page routes, under the Admin API's base. Reading needs
/// `posts:write`; a page is live once saved, so writing needs
/// `posts:publish`.
pub(crate) fn routes(pages: Kept<AdminPage>, door: AdminDoor) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(list_pages, create_page))
        .routes(routes!(read_page, edit_page, delete_page))
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

/// Reads one page. Needs `posts:write`.
#[utoipa::path(
    get, path = "/pages/{id}/", tag = "admin pages",
    security(("admin_key" = [])),
    params(("id" = String, Path, description = "The language and slug, such as en.about")),
    responses(
        (status = 200, description = "The page", body = AdminPages),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks posts:write", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "No page has this identity, or this server keeps no pages", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn read_page(
    State(pages): State<Kept<AdminPage>>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<AdminPages>, Problem> {
    require(&principal, Scope::PostsWrite)?;
    let found = PAGE.store(pages.as_deref())?.get(&id).await;
    let page = found.map_err(|error| Problem::internal(&error))?;
    let page = page.ok_or_else(|| PAGE.missing(uri.path()))?;
    Ok(Json(AdminPages { pages: vec![page] }))
}

/// A new page, wrapped the way the Admin API wraps pages: exactly one.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct NewPages {
    /// The one new page.
    pages: Vec<NewPage>,
}

/// Creates a page, live at once, in the language it names or the site's
/// default. Needs `posts:publish`.
#[utoipa::path(
    post, path = "/pages/", tag = "admin pages",
    security(("admin_key" = [])),
    request_body = NewPages,
    responses(
        (status = 201, description = "The new page", body = AdminPages),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks posts:publish", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "This server keeps no pages", body = Problem, content_type = "application/problem+json"),
        (status = 409, description = "Another page in that language has the slug", body = Problem, content_type = "application/problem+json"),
        (status = 422, description = "The body could not be read, the title is empty, or the site has no such language", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn create_page(
    State(pages): State<Kept<AdminPage>>,
    Extension(principal): Extension<Principal>,
    body: Result<Json<NewPages>, JsonRejection>,
) -> Result<(StatusCode, [(HeaderName, String); 1], Json<AdminPages>), Problem> {
    require(&principal, Scope::PostsPublish)?;
    let new = PAGE.one(PAGE.body(body)?.pages)?;
    titled(new.title())?;
    let made = PAGE.store(pages.as_deref())?.create(new).await;
    let page = made.map_err(|error| PAGE.problem(error, ""))?;
    let location = format!("{BASE}/pages/{}/", page.id());
    let pages = vec![page];
    Ok((
        StatusCode::CREATED,
        [(LOCATION, location)],
        Json(AdminPages { pages }),
    ))
}

/// An edit, wrapped the way the Admin API wraps pages: exactly one.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct PageEdits {
    /// The one edit.
    pages: Vec<PageEdit>,
}

/// Edits one page: only the fields sent change. The edit carries the
/// `updated_at` of the copy it was made on; when the page changed since,
/// the answer is a 409 (RULES.md BL-API-7). Needs `posts:publish`.
#[utoipa::path(
    put, path = "/pages/{id}/", tag = "admin pages",
    security(("admin_key" = [])),
    params(("id" = String, Path, description = "The language and slug, such as en.about")),
    request_body = PageEdits,
    responses(
        (status = 200, description = "The page as saved", body = AdminPages),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks posts:publish", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "No page has this identity, or this server keeps no pages", body = Problem, content_type = "application/problem+json"),
        (status = 409, description = "The page changed after the copy the edit was made on", body = Problem, content_type = "application/problem+json"),
        (status = 422, description = "The body could not be read, or the title is empty", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn edit_page(
    State(pages): State<Kept<AdminPage>>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    body: Result<Json<PageEdits>, JsonRejection>,
) -> Result<Json<AdminPages>, Problem> {
    require(&principal, Scope::PostsPublish)?;
    let edit = PAGE.one(PAGE.body(body)?.pages)?;
    edit.title().map(titled).transpose()?;
    let saved = PAGE.store(pages.as_deref())?.update(&id, edit).await;
    let page = saved.map_err(|error| PAGE.problem(error, uri.path()))?;
    Ok(Json(AdminPages { pages: vec![page] }))
}

/// Deletes one page. The request names the `updated_at` of the copy it
/// was made on; when the page changed since, the answer is a 409 (RULES.md
/// BL-API-7). Needs `posts:publish`.
#[utoipa::path(
    delete, path = "/pages/{id}/", tag = "admin pages",
    security(("admin_key" = [])),
    params(("id" = String, Path, description = "The language and slug, such as en.about"), DeleteQuery),
    responses(
        (status = 204, description = "The page is gone"),
        (status = 400, description = "updated_at is missing or is not a time", body = Problem, content_type = "application/problem+json"),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks posts:publish", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "No page has this identity, or this server keeps no pages", body = Problem, content_type = "application/problem+json"),
        (status = 409, description = "The page changed after the copy the delete was made on", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn delete_page(
    State(pages): State<Kept<AdminPage>>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    query: Result<Query<DeleteQuery>, QueryRejection>,
) -> Result<StatusCode, Problem> {
    require(&principal, Scope::PostsPublish)?;
    let seen = query.ok().and_then(|Query(query)| query.updated_at);
    let seen = seen.ok_or_else(|| PAGE.no_copy())?;
    let deleted = PAGE.store(pages.as_deref())?.delete(&id, seen).await;
    deleted.map_err(|error| PAGE.problem(error, uri.path()))?;
    Ok(StatusCode::NO_CONTENT)
}

/// Refuses an empty title.
fn titled(title: &str) -> Result<(), Problem> {
    if title.trim().is_empty() {
        return Err(PAGE.invalid("title must not be empty", Some("title")));
    }
    Ok(())
}
