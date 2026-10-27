//! The webhook routes of the Admin API under `/api/admin/v1/webhooks/`
//! (task 1.19, KH-API-3). Every route needs `webhooks:manage`.

use std::sync::Arc;

use axum::extract::State;
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
use crate::webhook_store::{StoredWebhook, WebhookStore};

/// The site's webhooks, or `None` for a site that keeps none.
pub(crate) type Webhooks = Option<Arc<dyn WebhookStore>>;

/// The webhook routes, each declared with `utoipa::path` (RULES.md
/// BL-API-4), behind an integration key or staff token.
pub(crate) fn routes(webhooks: Webhooks, door: AdminDoor) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(list_webhooks))
        .route_layer(middleware::from_fn_with_state(door, require_admin_key))
        .route_layer(middleware::from_fn(negotiate_version))
        .with_state(webhooks)
}

/// A webhook as the Admin API lists it. Its secret is never listed.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct AdminWebhook {
    /// The identity to remove it by.
    #[schema(example = 3)]
    id: i64,
    /// The event it sends, from the catalogue in `docs/standards/api.md`.
    #[schema(example = "post.published")]
    event: &'static str,
    /// Where it sends it.
    #[schema(example = "https://hooks.example.com/khalid")]
    url: String,
    /// When it was added.
    created_at: DateTime<Utc>,
}

impl From<&StoredWebhook> for AdminWebhook {
    fn from(stored: &StoredWebhook) -> Self {
        Self {
            id: stored.id,
            event: stored.event.as_str(),
            url: stored.url.as_str().to_owned(),
            created_at: stored.created_at,
        }
    }
}

/// Webhooks as the Admin API wraps them.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct AdminWebhooks {
    /// The webhooks, oldest first.
    webhooks: Vec<AdminWebhook>,
}

/// Lists every webhook that is switched on, oldest first, without their
/// secrets. Needs `webhooks:manage`.
#[utoipa::path(
    get, path = "/webhooks/", tag = "admin webhooks",
    security(("admin_key" = [])),
    responses(
        (status = 200, description = "Every webhook that is on", body = AdminWebhooks),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks webhooks:manage", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn list_webhooks(
    State(webhooks): State<Webhooks>,
    Extension(principal): Extension<Principal>,
) -> Result<Json<AdminWebhooks>, Problem> {
    require(&principal, Scope::WebhooksManage)?;
    let Some(store) = webhooks else {
        return Ok(Json(AdminWebhooks {
            webhooks: Vec::new(),
        }));
    };
    let stored = store.list().await.map_err(|e| Problem::internal(&e))?;
    let webhooks = stored.iter().map(AdminWebhook::from).collect();
    Ok(Json(AdminWebhooks { webhooks }))
}
