//! The Admin API's settings routes: the site's core settings in
//! `site.toml`, read and changed (plan section 5.12, KH-API-2).

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
use crate::resources::AdminSettings;
use crate::settings_store::SettingsStore;

/// How messages name the settings.
const SETTINGS: Noun = Noun {
    one: "setting",
    many: "settings",
    example: "name",
};

/// The site's settings, when the server keeps them.
type Settings = Option<Arc<dyn SettingsStore>>;

/// The settings routes, under the Admin API's base. Both need
/// `settings:write`, as the admin's Settings screen does.
pub(crate) fn routes(settings: Settings, door: AdminDoor) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(read_settings))
        .route_layer(middleware::from_fn_with_state(door, require_admin_key))
        .route_layer(middleware::from_fn(negotiate_version))
        .with_state(settings)
}

/// The settings as the Admin API wraps them.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SiteSettings {
    /// The site's core settings.
    settings: AdminSettings,
}

/// Reads the site's core settings as `site.toml` holds them, and whether
/// the running server needs a restart to use them. Needs `settings:write`.
#[utoipa::path(
    get, path = "/settings/", tag = "admin settings",
    security(("admin_key" = [])),
    responses(
        (status = 200, description = "The settings", body = SiteSettings),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks settings:write", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "This server keeps no settings", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn read_settings(
    State(settings): State<Settings>,
    Extension(principal): Extension<Principal>,
) -> Result<Json<SiteSettings>, Problem> {
    require(&principal, Scope::SettingsWrite)?;
    let read = SETTINGS.store(settings.as_deref())?.read().await;
    let settings = read.map_err(|error| Problem::internal(&error))?;
    Ok(Json(SiteSettings { settings }))
}
