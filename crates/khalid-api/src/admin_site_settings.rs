//! The Admin API's settings routes: the site's core settings in
//! `site.toml`, read and changed (plan section 5.12, KH-API-2).

use std::sync::Arc;

use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::{Extension, Json, middleware};
use chrono::{DateTime, Utc};
use khalid_core::UserMessage;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::admin_entries::Noun;
use crate::admin_key::{AdminDoor, require, require_admin_key};
use crate::api_version::negotiate_version;
use crate::keys::{Principal, Scope};
use crate::problem::{Problem, ProblemType};
use crate::resources::{AdminSettings, SettingsChange};
use crate::settings_store::{SettingsError, SettingsStore};

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
        .routes(routes!(read_settings, change_settings))
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

/// A change, wrapped the way the Admin API wraps the settings.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SettingsChanges {
    /// The change.
    settings: SettingsChange,
}

/// Changes the core settings in `site.toml`: only the fields sent change,
/// and each value must pass the checks `khalid serve` makes at start. The
/// change carries the `updated_at` of the copy it was made on; when
/// `site.toml` changed since, the answer is a 409 (RULES.md BL-API-7). The
/// running server reads `site.toml` at start, so `restart_needed` says
/// when a restart applies the change. Needs `settings:write`.
#[utoipa::path(
    put, path = "/settings/", tag = "admin settings",
    security(("admin_key" = [])),
    request_body = SettingsChanges,
    responses(
        (status = 200, description = "The settings as saved", body = SiteSettings),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks settings:write", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "This server keeps no settings", body = Problem, content_type = "application/problem+json"),
        (status = 409, description = "site.toml changed after the copy the change was made on", body = Problem, content_type = "application/problem+json"),
        (status = 422, description = "The body could not be read, or a value would not pass the checks at start", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn change_settings(
    State(settings): State<Settings>,
    Extension(principal): Extension<Principal>,
    body: Result<Json<SettingsChanges>, JsonRejection>,
) -> Result<Json<SiteSettings>, Problem> {
    require(&principal, Scope::SettingsWrite)?;
    let change = SETTINGS.body(body)?.settings;
    let changed = SETTINGS.store(settings.as_deref())?.change(change).await;
    let settings = changed.map_err(problem)?;
    Ok(Json(SiteSettings { settings }))
}

/// The problem a refused change answers with.
fn problem(error: SettingsError) -> Problem {
    match error {
        SettingsError::Stale { updated_at } => stale(updated_at),
        SettingsError::Refused { field, problem } => {
            let message = UserMessage::new(
                format!("site.toml cannot take this {field}: {problem}."),
                "Khalid checks every setting as it does at start, so the site would not start.",
                format!("Change {field} and send it again; nothing was saved."),
            );
            Problem::new(ProblemType::InvalidBody, &message).with_rule(&field)
        }
        error => Problem::internal(&error),
    }
}

fn stale(updated_at: DateTime<Utc>) -> Problem {
    let when = updated_at.format("%Y-%m-%d %H:%M UTC");
    let message = UserMessage::new(
        format!("site.toml changed at {when}, after the copy you have."),
        "Saving your copy would undo that change.",
        "Read the settings again, make your change to them, and send the new updated_at.",
    );
    Problem::new(ProblemType::Conflict, &message).with_rule("updated_at")
}
