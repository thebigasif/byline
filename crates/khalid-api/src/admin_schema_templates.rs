//! The schema template routes of the Admin API under
//! `/api/admin/v1/schema-templates/` (G54, KH-SEO-4). Both need
//! `settings:write`.

use std::sync::Arc;

use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::{Extension, Json, middleware};
use chrono::{DateTime, Utc};
use khalid_core::UserMessage;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::admin_key::{AdminDoor, require, require_admin_key};
use crate::api_version::negotiate_version;
use crate::keys::{Principal, Scope};
use crate::problem::{Problem, ProblemType};
use crate::schema_template_store::{
    SchemaTemplateStore, SchemaTemplates, TemplateStoreError, TemplatesChange,
};

/// The site's schema templates, or `None` for a server that keeps none.
pub(crate) type Templates = Option<Arc<dyn SchemaTemplateStore>>;

/// The schema template routes, each declared with `utoipa::path`
/// (RULES.md BL-API-4), behind an integration key or staff token.
pub(crate) fn routes(templates: Templates, door: AdminDoor) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(read_templates, replace_templates))
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
    store.read().await.map(Json).map_err(problem)
}

/// Puts a new list in place of every schema template, made on the copy
/// `updated_at` names. Each template is checked as `khalid serve` checks
/// `site.toml` at start, and nothing is saved when one is refused. The
/// running server uses the new templates after a restart. Needs
/// `settings:write`.
#[utoipa::path(
    put, path = "/schema-templates/", tag = "admin settings",
    security(("admin_key" = [])),
    request_body = TemplatesChange,
    responses(
        (status = 200, description = "The schema templates as saved", body = SchemaTemplates),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks settings:write", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "This server keeps no schema templates", body = Problem, content_type = "application/problem+json"),
        (status = 409, description = "The templates changed after the copy this change was made on", body = Problem, content_type = "application/problem+json"),
        (status = 422, description = "The body could not be read, or a template is not valid", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn replace_templates(
    State(templates): State<Templates>,
    Extension(principal): Extension<Principal>,
    body: Result<Json<TemplatesChange>, JsonRejection>,
) -> Result<Json<SchemaTemplates>, Problem> {
    require(&principal, Scope::SettingsWrite)?;
    let store = templates.ok_or_else(no_templates)?;
    let Json(change) = body.map_err(|rejection| refused(&rejection.body_text()))?;
    for (at, template) in change.templates.iter().enumerate() {
        template
            .checked()
            .map_err(|why| refused(&format!("template {}: {why}", at.saturating_add(1))))?;
    }
    store.replace(change).await.map(Json).map_err(problem)
}

fn problem(error: TemplateStoreError) -> Problem {
    match error {
        TemplateStoreError::Stale { updated_at } => stale(updated_at),
        other => Problem::internal(&other),
    }
}

fn stale(updated_at: DateTime<Utc>) -> Problem {
    let message = UserMessage::new(
        "Someone changed the schema templates while you were editing them.",
        format!("They last changed at {updated_at}, after the copy this change was made on."),
        "Load the templates again, make the change once more, and save.",
    );
    Problem::new(ProblemType::Conflict, &message).with_rule("updated_at")
}

fn refused(why: &str) -> Problem {
    let message = UserMessage::new(
        format!("The schema templates could not be read: {why}."),
        "A template Google cannot read would put broken schema on every post it covers.",
        "Correct the template the message names, then save again.",
    );
    Problem::new(ProblemType::InvalidBody, &message)
}

fn no_templates() -> Problem {
    let message = UserMessage::new(
        "This server keeps no schema templates.",
        "Schema templates live in the site's site.toml, which only the site's own server reads.",
        "Send the request to the running khalid serve.",
    );
    Problem::new(ProblemType::NotFound, &message)
}
