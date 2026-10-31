//! The schema template routes of the Admin API under
//! `/api/admin/v1/schema-templates/` (G54, KH-SEO-4). Both need
//! `settings:write`.

use std::sync::Arc;

use axum::extract::State;
use axum::{Extension, Json, middleware};
use khalid_core::UserMessage;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::admin_key::{AdminDoor, require, require_admin_key};
use crate::api_version::negotiate_version;
use crate::keys::{Principal, Scope};
use crate::problem::{Problem, ProblemType};
use crate::schema_template_store::{SchemaTemplateStore, SchemaTemplates};

/// The site's schema templates, or `None` for a server that keeps none.
pub(crate) type Templates = Option<Arc<dyn SchemaTemplateStore>>;

/// The schema template routes, each declared with `utoipa::path`
/// (RULES.md BL-API-4), behind an integration key or staff token.
pub(crate) fn routes(templates: Templates, door: AdminDoor) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(read_templates))
        .route_layer(middleware::from_fn_with_state(door, require_admin_key))
        .route_layer(middleware::from_fn(negotiate_version))
        .with_state(templates)
}

/// Lists the site's schema templates, the collections a template may name,
/// and whether the running server needs a restart to use the templates.
/// Needs `settings:write`.
#[utoipa::path(
    get, path = "/schema-templates/", tag = "admin settings",
    security(("admin_key" = [])),
    responses(
        (status = 200, description = "The schema templates", body = SchemaTemplates),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks settings:write", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "This server keeps no schema templates", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn read_templates(
    State(templates): State<Templates>,
    Extension(principal): Extension<Principal>,
) -> Result<Json<SchemaTemplates>, Problem> {
    require(&principal, Scope::SettingsWrite)?;
    let store = templates.ok_or_else(no_templates)?;
    let read = store.read().await.map_err(|e| Problem::internal(&e))?;
    Ok(Json(read))
}

fn no_templates() -> Problem {
    let message = UserMessage::new(
        "This server keeps no schema templates.",
        "Schema templates live in the site's site.toml, which only the site's own server reads.",
        "Send the request to the running khalid serve.",
    );
    Problem::new(ProblemType::NotFound, &message)
}
