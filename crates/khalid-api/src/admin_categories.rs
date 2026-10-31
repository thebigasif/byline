//! The Admin API's category routes: the groups posts belong to, each named
//! in every language (plan sections 4.4 and 5.12, KH-API-2).

use std::collections::BTreeMap;

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
use crate::resources::{AdminCategory, CategoryEdit, NewCategory, names_problem};

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
        .routes(routes!(list_categories, create_category))
        .routes(routes!(read_category, edit_category, delete_category))
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

/// A new category, wrapped the way the Admin API wraps categories: exactly one.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct NewCategories {
    /// The one new category.
    categories: Vec<NewCategory>,
}

/// Creates a category, named in one or more of the site's languages.
/// Needs `posts:publish`.
#[utoipa::path(
    post, path = "/categories/", tag = "admin categories",
    security(("admin_key" = [])),
    request_body = NewCategories,
    responses(
        (status = 201, description = "The new category", body = AdminCategories),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks posts:publish", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "This server keeps no categories", body = Problem, content_type = "application/problem+json"),
        (status = 409, description = "Another category has the slug", body = Problem, content_type = "application/problem+json"),
        (status = 422, description = "The body could not be read, or a name is missing, empty or not in one of the site's languages", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn create_category(
    State(categories): State<Kept<AdminCategory>>,
    Extension(principal): Extension<Principal>,
    body: Result<Json<NewCategories>, JsonRejection>,
) -> Result<(StatusCode, [(HeaderName, String); 1], Json<AdminCategories>), Problem> {
    require(&principal, Scope::PostsPublish)?;
    let new = CATEGORY.one(CATEGORY.body(body)?.categories)?;
    named(new.names())?;
    let made = CATEGORY.store(categories.as_deref())?.create(new).await;
    let category = made.map_err(|error| CATEGORY.problem(error, ""))?;
    let location = format!("{BASE}/categories/{}/", category.id());
    let categories = vec![category];
    Ok((
        StatusCode::CREATED,
        [(LOCATION, location)],
        Json(AdminCategories { categories }),
    ))
}
/// Reads one category. Needs `posts:write`.
#[utoipa::path(
    get, path = "/categories/{id}/", tag = "admin categories",
    security(("admin_key" = [])),
    params(("id" = String, Path, description = "The slug, such as on-page-seo")),
    responses(
        (status = 200, description = "The category", body = AdminCategories),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks posts:write", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "No category has this identity, or this server keeps no categories", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn read_category(
    State(categories): State<Kept<AdminCategory>>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<AdminCategories>, Problem> {
    require(&principal, Scope::PostsWrite)?;
    let found = CATEGORY.store(categories.as_deref())?.get(&id).await;
    let category = found.map_err(|error| Problem::internal(&error))?;
    let category = category.ok_or_else(|| CATEGORY.missing(uri.path()))?;
    Ok(Json(AdminCategories {
        categories: vec![category],
    }))
}

/// An edit, wrapped the way the Admin API wraps categories: exactly one.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct CategoryEdits {
    /// The one edit.
    categories: Vec<CategoryEdit>,
}

/// Renames one category: the names sent replace all of its names, and
/// every post in it shows them. The edit carries the `updated_at` of the
/// copy it was made on; when the category changed since, the answer is a
/// 409 (RULES.md BL-API-7). Needs `posts:publish`.
#[utoipa::path(
    put, path = "/categories/{id}/", tag = "admin categories",
    security(("admin_key" = [])),
    params(("id" = String, Path, description = "The slug, such as on-page-seo")),
    request_body = CategoryEdits,
    responses(
        (status = 200, description = "The category as saved", body = AdminCategories),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks posts:publish", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "No category has this identity, or this server keeps no categories", body = Problem, content_type = "application/problem+json"),
        (status = 409, description = "The categories changed after the copy the edit was made on", body = Problem, content_type = "application/problem+json"),
        (status = 422, description = "The body could not be read, or a name is missing, empty or not in one of the site's languages", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn edit_category(
    State(categories): State<Kept<AdminCategory>>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    body: Result<Json<CategoryEdits>, JsonRejection>,
) -> Result<Json<AdminCategories>, Problem> {
    require(&principal, Scope::PostsPublish)?;
    let edit = CATEGORY.one(CATEGORY.body(body)?.categories)?;
    named(edit.names())?;
    let saved = CATEGORY
        .store(categories.as_deref())?
        .update(&id, edit)
        .await;
    let category = saved.map_err(|error| CATEGORY.problem(error, uri.path()))?;
    Ok(Json(AdminCategories {
        categories: vec![category],
    }))
}

/// Deletes one category no post is in. The request names the `updated_at`
/// of the copy it was made on; when the categories changed since, or posts
/// are still in it, the answer is a 409. Needs `posts:publish`.
#[utoipa::path(
    delete, path = "/categories/{id}/", tag = "admin categories",
    security(("admin_key" = [])),
    params(("id" = String, Path, description = "The slug, such as on-page-seo"), DeleteQuery),
    responses(
        (status = 204, description = "The category is gone"),
        (status = 400, description = "updated_at is missing or is not a time", body = Problem, content_type = "application/problem+json"),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks posts:publish", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "No category has this identity, or this server keeps no categories", body = Problem, content_type = "application/problem+json"),
        (status = 409, description = "The categories changed after the copy the delete was made on, or posts are still in the category", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn delete_category(
    State(categories): State<Kept<AdminCategory>>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    query: Result<Query<DeleteQuery>, QueryRejection>,
) -> Result<StatusCode, Problem> {
    require(&principal, Scope::PostsPublish)?;
    let seen = query.ok().and_then(|Query(query)| query.updated_at);
    let seen = seen.ok_or_else(|| CATEGORY.no_copy())?;
    let deleted = CATEGORY
        .store(categories.as_deref())?
        .delete(&id, seen)
        .await;
    deleted.map_err(|error| CATEGORY.problem(error, uri.path()))?;
    Ok(StatusCode::NO_CONTENT)
}

/// Refuses names a category cannot be kept with, naming `names`.
fn named(names: &BTreeMap<String, String>) -> Result<(), Problem> {
    match names_problem(names) {
        Some(reason) => Err(CATEGORY.invalid(reason, Some("names"))),
        None => Ok(()),
    }
}
