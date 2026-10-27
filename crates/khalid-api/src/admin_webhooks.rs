//! The webhook routes of the Admin API under `/api/admin/v1/webhooks/`
//! (task 1.19, KH-API-3). Every route needs `webhooks:manage`.

use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::{Extension, Json, middleware};
use chrono::{DateTime, Utc};
use khalid_core::{AbsoluteUrl, UserMessage};
use khalid_webhooks::Event;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::admin_key::{AdminDoor, require, require_admin_key};
use crate::api_version::negotiate_version;
use crate::keys::{Principal, Scope};
use crate::problem::{Problem, ProblemType};
use crate::webhook_store::{StoredWebhook, WebhookStore};

/// The site's webhooks, or `None` for a site that keeps none.
pub(crate) type Webhooks = Option<Arc<dyn WebhookStore>>;

/// The webhook routes, each declared with `utoipa::path` (RULES.md
/// BL-API-4), behind an integration key or staff token.
pub(crate) fn routes(webhooks: Webhooks, door: AdminDoor) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(list_webhooks, add_webhook))
        .routes(routes!(remove_webhook))
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
    /// The secret every delivery is signed with. Only the answer that adds
    /// the webhook carries it, once.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "6f1c0e9a2b7d4c8e9f0a1b2c3d4e5f60718293a4b5c6d7e8f9012a3b4c5d6e7f")]
    secret: Option<String>,
}

impl From<&StoredWebhook> for AdminWebhook {
    fn from(stored: &StoredWebhook) -> Self {
        Self {
            id: stored.id,
            event: stored.event.as_str(),
            url: stored.url.as_str().to_owned(),
            created_at: stored.created_at,
            secret: None,
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

/// One new webhook.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct NewWebhook {
    /// The event to send, such as `post.published`.
    #[schema(example = "post.published")]
    event: String,
    /// Where to send it: an `http` or `https` address.
    #[schema(example = "https://hooks.example.com/khalid")]
    url: String,
}

/// The body of a new webhook: exactly one.
#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct NewWebhooks {
    /// The one webhook to add.
    webhooks: Vec<NewWebhook>,
}

/// Adds a webhook that sends one event to one address, signed with a new
/// secret. The answer shows the secret once; keep it, because the
/// receiver needs it to check each delivery. Needs `webhooks:manage`.
#[utoipa::path(
    post, path = "/webhooks/", tag = "admin webhooks",
    security(("admin_key" = [])),
    request_body = NewWebhooks,
    responses(
        (status = 201, description = "The new webhook, with its secret this once", body = AdminWebhooks),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks webhooks:manage", body = Problem, content_type = "application/problem+json"),
        (status = 422, description = "The event is not in the catalogue, the address cannot be sent to, or the site keeps no webhooks", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn add_webhook(
    State(webhooks): State<Webhooks>,
    Extension(principal): Extension<Principal>,
    body: Result<Json<NewWebhooks>, JsonRejection>,
) -> Result<(StatusCode, Json<AdminWebhooks>), Problem> {
    require(&principal, Scope::WebhooksManage)?;
    let store = webhooks.ok_or_else(no_webhooks)?;
    let Json(body) = body.map_err(|rejection| one_webhook(&rejection.body_text()))?;
    let [new] = <[NewWebhook; 1]>::try_from(body.webhooks)
        .map_err(|_| one_webhook("webhooks must hold exactly one webhook"))?;
    let event = new
        .event
        .parse::<Event>()
        .map_err(|_| unknown_event(&new.event))?;
    let url = AbsoluteUrl::parse(&new.url).map_err(|_| bad_url())?;
    let (stored, secret) = store
        .add(&url, event)
        .await
        .map_err(|e| Problem::internal(&e))?;
    let added = AdminWebhook {
        secret: Some(secret.expose().clone()),
        ..AdminWebhook::from(&stored)
    };
    let webhooks = vec![added];
    Ok((StatusCode::CREATED, Json(AdminWebhooks { webhooks })))
}

/// Switches a webhook off: nothing more is sent to it, and its past
/// deliveries stay in the log. Needs `webhooks:manage`.
#[utoipa::path(
    delete, path = "/webhooks/{id}/", tag = "admin webhooks",
    security(("admin_key" = [])),
    params(("id" = i64, Path, description = "The webhook's identity")),
    responses(
        (status = 204, description = "The webhook is off"),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks webhooks:manage", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "No webhook that is on has this identity", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn remove_webhook(
    State(webhooks): State<Webhooks>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
) -> Result<StatusCode, Problem> {
    require(&principal, Scope::WebhooksManage)?;
    // An identity that is not a number names no webhook, so it gets the
    // same 404 as one already switched off.
    let removed = match (webhooks, id.parse::<i64>()) {
        (Some(store), Ok(id)) => store.remove(id).await.map_err(|e| Problem::internal(&e))?,
        _ => false,
    };
    if !removed {
        let message = UserMessage::new(
            "No webhook that is on has this identity.",
            "It may have been switched off already.",
            "List the webhooks to find the one you want.",
        );
        return Err(Problem::new(ProblemType::NotFound, &message));
    }
    Ok(StatusCode::NO_CONTENT)
}

/// The 422 for a body that does not hold exactly one webhook.
fn one_webhook(reason: &str) -> Problem {
    let message = UserMessage::new(
        format!("This webhook cannot be added: {reason}."),
        "Each request adds one webhook, which sends one event to one address.",
        "Send webhooks with one entry holding an event and a url.",
    );
    Problem::new(ProblemType::InvalidBody, &message).with_rule("webhooks")
}

/// The 422 for an event name that is not in the catalogue.
fn unknown_event(name: &str) -> Problem {
    let message = UserMessage::new(
        format!("{name} is not a webhook event."),
        "Khalid sends only the events in its catalogue, so this webhook would never fire.",
        "Use a name from the catalogue, such as post.published or comment.added.",
    );
    Problem::new(ProblemType::InvalidBody, &message).with_rule("event")
}

/// The 422 for an address deliveries cannot be sent to. The address is not
/// repeated, since it may carry a password.
fn bad_url() -> Problem {
    let message = UserMessage::new(
        "The url is not an address Khalid can send to.",
        "Deliveries go over the web, so the address needs http or https, a host, \
         and no user name or password.",
        "Give the full address, such as https://hooks.example.com/khalid.",
    );
    Problem::new(ProblemType::InvalidBody, &message).with_rule("url")
}

/// The 422 for a site that keeps no webhooks.
fn no_webhooks() -> Problem {
    let message = UserMessage::new(
        "This site keeps no webhooks.",
        "Webhooks are kept by the site's server, in its database.",
        "Send the request to the running khalid serve.",
    );
    Problem::new(ProblemType::InvalidBody, &message)
}
