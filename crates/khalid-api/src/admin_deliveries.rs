//! What one webhook sent, `GET /api/admin/v1/webhooks/{id}/deliveries/`
//! (task 4.10, KH-API-3). The webhook routes serve it.

use axum::extract::{Path, State};
use axum::{Extension, Json};
use chrono::{DateTime, Utc};
use khalid_webhooks::{Attempt, Delivery, DeliveryState, Outcome};
use serde::Serialize;
use utoipa::ToSchema;

use crate::admin_key::require;
use crate::admin_webhooks::{Webhooks, not_on};
use crate::keys::{Principal, Scope};
use crate::problem::Problem;

/// How many deliveries the log lists: the newest ones.
const LISTED: u32 = 50;

/// One try at sending a delivery: the receiver's status, or why it could
/// not be reached. Exactly one of the two is there.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct AdminAttempt {
    /// When it was made.
    at: DateTime<Utc>,
    /// The HTTP status the receiver answered; 200 to 299 is a success.
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<u16>,
    /// Why the receiver could not be reached.
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
}

/// Where a delivery stands: waiting for its next try, delivered, or given
/// up after every try failed.
#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum StateName {
    Pending,
    Delivered,
    GaveUp,
}

/// One message a webhook sent, with every try.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct AdminDelivery {
    /// Its place in the log: a later delivery has a larger number.
    id: i64,
    /// The event it announced, such as `post.published`.
    event: &'static str,
    /// Where it stands.
    state: StateName,
    /// When the next try is due, or when it was delivered or given up.
    state_at: DateTime<Utc>,
    /// Every try so far, oldest first.
    attempts: Vec<AdminAttempt>,
}

/// The newest deliveries of one webhook.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct AdminDeliveries {
    /// At most 50, newest first.
    deliveries: Vec<AdminDelivery>,
}

/// A delivery this version does not know how to show.
#[derive(Debug, thiserror::Error)]
#[error("a webhook delivery is in a shape this version cannot show")]
struct UnknownShape;

/// Lists the 50 newest messages one webhook that is on sent, newest
/// first, each with every try and where it stands. The bodies sent are
/// left out. Needs `webhooks:manage`.
#[utoipa::path(
    get, path = "/webhooks/{id}/deliveries/", tag = "admin webhooks",
    security(("admin_key" = [])),
    params(("id" = i64, Path, description = "The webhook's identity")),
    responses(
        (status = 200, description = "The webhook's newest deliveries", body = AdminDeliveries),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks webhooks:manage", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "No webhook that is on has this identity", body = Problem, content_type = "application/problem+json"),
    )
)]
pub(crate) async fn list_deliveries(
    State(webhooks): State<Webhooks>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
) -> Result<Json<AdminDeliveries>, Problem> {
    require(&principal, Scope::WebhooksManage)?;
    let (Some(store), Ok(id)) = (webhooks, id.parse::<i64>()) else {
        return Err(not_on());
    };
    let on = store.list().await.map_err(|e| Problem::internal(&e))?;
    if !on.iter().any(|webhook| webhook.id == id) {
        return Err(not_on());
    }
    let log = store.deliveries(id, LISTED).await;
    let log = log.map_err(|e| Problem::internal(&e))?;
    let deliveries = log.iter().map(shown).collect::<Result<_, _>>();
    let deliveries = deliveries.map_err(|e| Problem::internal(&e))?;
    Ok(Json(AdminDeliveries { deliveries }))
}

/// `delivery` as the Admin API shows it.
fn shown(delivery: &Delivery) -> Result<AdminDelivery, UnknownShape> {
    let (state, state_at) = match *delivery.state() {
        DeliveryState::Pending { next_at } => (StateName::Pending, next_at),
        DeliveryState::Delivered { at } => (StateName::Delivered, at),
        DeliveryState::GaveUp { at } => (StateName::GaveUp, at),
        _ => return Err(UnknownShape),
    };
    let attempts = delivery.attempts().iter().map(attempt);
    Ok(AdminDelivery {
        id: delivery.id().get(),
        event: delivery.event().as_str(),
        state,
        state_at,
        attempts: attempts.collect::<Result<_, _>>()?,
    })
}

/// `attempt` as the Admin API shows it.
fn attempt(attempt: &Attempt) -> Result<AdminAttempt, UnknownShape> {
    let (status, reason) = match attempt.outcome() {
        Outcome::Answered { status } => (Some(*status), None),
        Outcome::Unreachable { reason } => (None, Some(reason.clone())),
        _ => return Err(UnknownShape),
    };
    let at = attempt.at();
    Ok(AdminAttempt { at, status, reason })
}
