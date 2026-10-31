//! The `SerpDad` route of the Admin API under `/api/admin/v1/serpdad/`,
//! for the Connect to `SerpDad` page (KH-API-6,
//! `docs/decisions/2026-10-31-connect-to-serpdad.md`). Every method needs
//! `settings:write`.

use std::sync::Arc;

use axum::{Extension, Json, middleware};
use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::admin_key::{AdminDoor, require, require_admin_key};
use crate::api_version::negotiate_version;
use crate::keys::{Principal, Scope};
use crate::problem::Problem;
use crate::serpdad_store::{SerpdadLink, SerpdadStore};

/// The site's `SerpDad` connection, or `None` for a site with no database.
#[derive(Clone)]
pub(crate) struct Serpdad {
    store: Option<Arc<dyn SerpdadStore>>,
}

impl Serpdad {
    /// The connection kept in `store`.
    pub(crate) fn new(store: Option<Arc<dyn SerpdadStore>>) -> Self {
        Self { store }
    }
}

/// The `SerpDad` route, declared with `utoipa::path` (RULES.md BL-API-4),
/// behind an integration key or staff token.
pub(crate) fn routes(serpdad: Serpdad, door: AdminDoor) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(read_serpdad))
        .route_layer(middleware::from_fn_with_state(door, require_admin_key))
        .route_layer(middleware::from_fn(negotiate_version))
        .layer(Extension(serpdad))
}

/// The connection, or `null` while it is off.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SerpdadConnection {
    /// When the key `SerpDad` sends drafts with was made.
    connected_at: DateTime<Utc>,
}

impl From<&SerpdadLink> for SerpdadConnection {
    fn from(link: &SerpdadLink) -> Self {
        Self {
            connected_at: link.connected_at,
        }
    }
}

/// Whether `SerpDad` is connected.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SerpdadState {
    /// The connection, or `null` while it is off.
    connection: Option<SerpdadConnection>,
}

/// Reads whether `SerpDad` is connected. Needs `settings:write`.
#[utoipa::path(
    get, path = "/serpdad/", tag = "admin serpdad",
    security(("admin_key" = [])),
    responses(
        (status = 200, description = "The connection", body = SerpdadState),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks settings:write", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn read_serpdad(
    Extension(serpdad): Extension<Serpdad>,
    Extension(principal): Extension<Principal>,
) -> Result<Json<SerpdadState>, Problem> {
    require(&principal, Scope::SettingsWrite)?;
    let link = match &serpdad.store {
        Some(store) => store.link().await.map_err(|e| Problem::internal(&e))?,
        None => None,
    };
    let connection = link.as_ref().map(SerpdadConnection::from);
    Ok(Json(SerpdadState { connection }))
}
