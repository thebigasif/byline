//! The Admin API's key routes: the site's API keys, listed, made and
//! revoked (plan sections 4.6 and 5.12, KH-API-2, ADR 0011).

use std::sync::Arc;

use axum::extract::State;
use axum::{Extension, Json, middleware};
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::admin_key::{AdminDoor, require, require_admin_key};
use crate::api_version::negotiate_version;
use crate::keys::{KeyStore, Principal, Scope};
use crate::problem::Problem;
use crate::resources::AdminKey;

/// The site's keys, the same store every request's key is checked in.
type Keys = Arc<dyn KeyStore>;

/// The key routes, under the Admin API's base. A key opens the site, so
/// every route needs `admin:all`, which owners and admins hold.
pub(crate) fn routes(keys: Keys, door: AdminDoor) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(list_keys))
        .route_layer(middleware::from_fn_with_state(door, require_admin_key))
        .route_layer(middleware::from_fn(negotiate_version))
        .with_state(keys)
}

/// Keys as the Admin API wraps them.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct AdminKeys {
    /// The key or keys.
    keys: Vec<AdminKey>,
}

/// Lists every key that works, oldest first, with what each may do and
/// never its secret. Needs `admin:all`.
#[utoipa::path(
    get, path = "/keys/", tag = "admin keys",
    security(("admin_key" = [])),
    responses(
        (status = 200, description = "Every key that works", body = AdminKeys),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks admin:all", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn list_keys(
    State(keys): State<Keys>,
    Extension(principal): Extension<Principal>,
) -> Result<Json<AdminKeys>, Problem> {
    require(&principal, Scope::AdminAll)?;
    let listed = keys
        .list()
        .await
        .map_err(|error| Problem::internal(&error))?;
    let keys = listed.into_iter().map(AdminKey::from).collect();
    Ok(Json(AdminKeys { keys }))
}
