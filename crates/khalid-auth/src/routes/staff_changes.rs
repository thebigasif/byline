//! Changing a staff member's role, removing them, and handing the site to
//! another account, over the Admin API.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use khalid_api::keys::{Principal, Role};
use khalid_api::problem::{Problem, ProblemType};
use khalid_core::UserMessage;
use serde::Deserialize;
use utoipa::ToSchema;

use super::staff::{StaffList, acting_role, forbidden, invalid};
use super::{cookie_token, problem};
use crate::{Account, AccountId, Accounts, may_assign, may_manage};

/// A new role for an account.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct RoleChange {
    /// `admin`, `editor`, `writer` or `contributor`.
    role: String,
}

/// Gives an account another role. The caller must manage both its old
/// role and the new one.
#[utoipa::path(
    put, path = "/staff/{id}/", tag = "staff",
    security(("staff_session" = []), ("admin_key" = [])),
    params(("id" = i64, Path, description = "The account's number")),
    request_body = RoleChange,
    responses(
        (status = 200, description = "The account with its new role", body = StaffList),
        (status = 403, description = "This role cannot make that change", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "No account has this number", body = Problem, content_type = "application/problem+json"),
        (status = 422, description = "The body could not be read", body = Problem, content_type = "application/problem+json"),
    )
)]
pub(super) async fn change_role(
    State(accounts): State<Accounts>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<i64>,
    body: Result<Json<RoleChange>, JsonRejection>,
) -> Result<Json<StaffList>, Problem> {
    let fix = "Send JSON with role, one of admin, editor, writer or contributor.";
    let Json(body) = body.map_err(|_| invalid(fix))?;
    let role: Role = body.role.parse().map_err(|_| invalid(fix))?;
    let target = found(&accounts, id).await?;
    if !acting_role(&principal).is_some_and(|actor| may_assign(actor, target.role(), role)) {
        return Err(forbidden(
            "This role cannot make that change to that account.",
        ));
    }
    let store = accounts.store();
    store
        .set_role(target.id(), role)
        .await
        .map_err(|error| Problem::internal(&error))?;
    let changed = found(&accounts, id).await?;
    Ok(Json(StaffList::of(&changed)))
}

/// Removes an account, with its sessions and passkeys. The caller must
/// manage its role.
#[utoipa::path(
    delete, path = "/staff/{id}/", tag = "staff",
    security(("staff_session" = []), ("admin_key" = [])),
    params(("id" = i64, Path, description = "The account's number")),
    responses(
        (status = 204, description = "Removed"),
        (status = 403, description = "This role cannot remove that account", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "No account has this number", body = Problem, content_type = "application/problem+json"),
    )
)]
pub(super) async fn remove_staff(
    State(accounts): State<Accounts>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<i64>,
) -> Result<StatusCode, Problem> {
    let target = found(&accounts, id).await?;
    if !acting_role(&principal).is_some_and(|actor| may_manage(actor, target.role())) {
        return Err(forbidden("This role cannot remove that account."));
    }
    let store = accounts.store();
    store
        .remove(target.id())
        .await
        .map_err(|error| Problem::internal(&error))?;
    Ok(StatusCode::NO_CONTENT)
}

/// Hands the site to another account: it becomes the owner and the owner
/// until now becomes an admin. Only the owner, signed in with the session
/// cookie, may do this; no key can.
#[utoipa::path(
    post, path = "/staff/{id}/owner/", tag = "staff",
    security(("staff_session" = [])),
    params(("id" = i64, Path, description = "The account that becomes the owner")),
    responses(
        (status = 200, description = "The new owner", body = StaffList),
        (status = 403, description = "Only the signed-in owner may hand the site over", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "No other account has this number", body = Problem, content_type = "application/problem+json"),
    )
)]
pub(super) async fn transfer(
    State(accounts): State<Accounts>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Response {
    let owner = match cookie_token(&headers) {
        Some(token) => accounts.session(&token).await,
        None => return forbidden(ONLY_OWNER).into_response(),
    };
    let owner = match owner {
        Ok(signed_in) if signed_in.account.role() == Role::Owner => signed_in.account,
        Ok(_) => return forbidden(ONLY_OWNER).into_response(),
        Err(error) => return problem(&error),
    };
    let store = accounts.store();
    match store.transfer(owner.id(), AccountId::from_row(id)).await {
        Ok(true) => match found(&accounts, id).await {
            Ok(new) => Json(StaffList::of(&new)).into_response(),
            Err(missing) => missing.into_response(),
        },
        Ok(false) => missing().into_response(),
        Err(error) => Problem::internal(&error).into_response(),
    }
}

const ONLY_OWNER: &str = "Only the owner, signed in, may hand the site to someone else.";

/// The account numbered `id`, or a 404.
async fn found(accounts: &Accounts, id: i64) -> Result<Account, Problem> {
    let account = accounts.store().find(AccountId::from_row(id)).await;
    account
        .map_err(|error| Problem::internal(&error))?
        .ok_or_else(missing)
}

fn missing() -> Problem {
    let message = UserMessage::new(
        "No other staff account has this number.",
        "The account may have been removed.",
        "Reload the staff list and pick the account again.",
    );
    Problem::new(ProblemType::NotFound, &message)
}
