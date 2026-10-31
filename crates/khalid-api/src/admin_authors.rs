//! The Admin API's author routes: the people bylines, author pages and the
//! author schema name (plan sections 4.4 and 5.12, KH-API-2).

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
use crate::resources::{AdminAuthor, AuthorEdit, NewAuthor};

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
        .routes(routes!(list_authors, create_author))
        .routes(routes!(read_author, edit_author, delete_author))
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

/// A new author, wrapped the way the Admin API wraps authors: exactly one.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct NewAuthors {
    /// The one new author.
    authors: Vec<NewAuthor>,
}

/// Creates an author in the language it names or the site's default.
/// Needs `posts:publish`.
#[utoipa::path(
    post, path = "/authors/", tag = "admin authors",
    security(("admin_key" = [])),
    request_body = NewAuthors,
    responses(
        (status = 201, description = "The new author", body = AdminAuthors),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks posts:publish", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "This server keeps no authors", body = Problem, content_type = "application/problem+json"),
        (status = 409, description = "Another author in that language has the slug", body = Problem, content_type = "application/problem+json"),
        (status = 422, description = "The body could not be read, the name is empty, or the site has no such language", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn create_author(
    State(authors): State<Kept<AdminAuthor>>,
    Extension(principal): Extension<Principal>,
    body: Result<Json<NewAuthors>, JsonRejection>,
) -> Result<(StatusCode, [(HeaderName, String); 1], Json<AdminAuthors>), Problem> {
    require(&principal, Scope::PostsPublish)?;
    let new = AUTHOR.one(AUTHOR.body(body)?.authors)?;
    AUTHOR.filled("name", new.name())?;
    let made = AUTHOR.store(authors.as_deref())?.create(new).await;
    let author = made.map_err(|error| AUTHOR.problem(error, ""))?;
    let location = format!("{BASE}/authors/{}/", author.id());
    let authors = vec![author];
    Ok((
        StatusCode::CREATED,
        [(LOCATION, location)],
        Json(AdminAuthors { authors }),
    ))
}
/// Reads one author. Needs `posts:write`.
#[utoipa::path(
    get, path = "/authors/{id}/", tag = "admin authors",
    security(("admin_key" = [])),
    params(("id" = String, Path, description = "The language and slug, such as en.asif")),
    responses(
        (status = 200, description = "The author", body = AdminAuthors),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks posts:write", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "No author has this identity, or this server keeps no authors", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn read_author(
    State(authors): State<Kept<AdminAuthor>>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<AdminAuthors>, Problem> {
    require(&principal, Scope::PostsWrite)?;
    let found = AUTHOR.store(authors.as_deref())?.get(&id).await;
    let author = found.map_err(|error| Problem::internal(&error))?;
    let author = author.ok_or_else(|| AUTHOR.missing(uri.path()))?;
    Ok(Json(AdminAuthors {
        authors: vec![author],
    }))
}

/// An edit, wrapped the way the Admin API wraps authors: exactly one.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct AuthorEdits {
    /// The one edit.
    authors: Vec<AuthorEdit>,
}

/// Edits one author: only the fields sent change, and every post that
/// names them shows the change. The edit carries the `updated_at` of the
/// copy it was made on; when the author changed since, the answer is a
/// 409 (RULES.md BL-API-7). Needs `posts:publish`.
#[utoipa::path(
    put, path = "/authors/{id}/", tag = "admin authors",
    security(("admin_key" = [])),
    params(("id" = String, Path, description = "The language and slug, such as en.asif")),
    request_body = AuthorEdits,
    responses(
        (status = 200, description = "The author as saved", body = AdminAuthors),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks posts:publish", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "No author has this identity, or this server keeps no authors", body = Problem, content_type = "application/problem+json"),
        (status = 409, description = "The author changed after the copy the edit was made on", body = Problem, content_type = "application/problem+json"),
        (status = 422, description = "The body could not be read, or the name is empty", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn edit_author(
    State(authors): State<Kept<AdminAuthor>>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    body: Result<Json<AuthorEdits>, JsonRejection>,
) -> Result<Json<AdminAuthors>, Problem> {
    require(&principal, Scope::PostsPublish)?;
    let edit = AUTHOR.one(AUTHOR.body(body)?.authors)?;
    edit.name()
        .map(|name| AUTHOR.filled("name", name))
        .transpose()?;
    let saved = AUTHOR.store(authors.as_deref())?.update(&id, edit).await;
    let author = saved.map_err(|error| AUTHOR.problem(error, uri.path()))?;
    Ok(Json(AdminAuthors {
        authors: vec![author],
    }))
}

/// Deletes one author. A post that still names them loses its byline.
/// The request names the `updated_at` of the copy it was made on; when the
/// author changed since, the answer is a 409. Needs `posts:publish`.
#[utoipa::path(
    delete, path = "/authors/{id}/", tag = "admin authors",
    security(("admin_key" = [])),
    params(("id" = String, Path, description = "The language and slug, such as en.asif"), DeleteQuery),
    responses(
        (status = 204, description = "The author is gone"),
        (status = 400, description = "updated_at is missing or is not a time", body = Problem, content_type = "application/problem+json"),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks posts:publish", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "No author has this identity, or this server keeps no authors", body = Problem, content_type = "application/problem+json"),
        (status = 409, description = "The author changed after the copy the delete was made on", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn delete_author(
    State(authors): State<Kept<AdminAuthor>>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    query: Result<Query<DeleteQuery>, QueryRejection>,
) -> Result<StatusCode, Problem> {
    require(&principal, Scope::PostsPublish)?;
    let seen = query.ok().and_then(|Query(query)| query.updated_at);
    let seen = seen.ok_or_else(|| AUTHOR.no_copy())?;
    let deleted = AUTHOR.store(authors.as_deref())?.delete(&id, seen).await;
    deleted.map_err(|error| AUTHOR.problem(error, uri.path()))?;
    Ok(StatusCode::NO_CONTENT)
}
