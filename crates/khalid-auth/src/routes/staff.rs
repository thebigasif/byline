//! Managing staff accounts over the Admin API: listing and inviting, within
//! what the caller's role may manage.

use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::StatusCode;
use axum::{Extension, Json};
use khalid_api::keys::{Access, Principal, Role, Scope};
use khalid_api::problem::{Problem, ProblemType};
use khalid_core::UserMessage;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::StaffAccount;
use crate::{Account, Accounts, CreateAccountError, Email, Permission, may_manage};

/// Staff accounts, as the Admin API wraps them.
#[derive(Debug, Serialize, ToSchema)]
pub(super) struct StaffList {
    /// The accounts, by email.
    staff: Vec<StaffAccount>,
}

impl StaffList {
    /// One account, wrapped the way lists are.
    pub(super) fn of(account: &Account) -> Self {
        Self {
            staff: vec![StaffAccount::from(account)],
        }
    }
}

/// A person to invite: they sign in by a link sent to their email, then
/// choose a password or add a passkey.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct Invite {
    /// Their email address.
    email: String,
    /// The name the admin shows.
    name: String,
    /// `admin`, `editor`, `writer` or `contributor`.
    role: String,
}

/// The role a caller acts with: a staff token's or session's role, and
/// an integration key with `admin:all` as an admin. Other keys manage no
/// one.
pub(super) fn acting_role(principal: &Principal) -> Option<Role> {
    match principal.access() {
        Access::Staff(role) => Some(*role),
        access if access.allows(Scope::AdminAll) => Some(Role::Admin),
        _ => None,
    }
}

/// Lists every staff account. Needs a role that manages staff.
#[utoipa::path(
    get, path = "/staff/", tag = "staff",
    security(("staff_session" = []), ("admin_key" = [])),
    responses(
        (status = 200, description = "Every staff account", body = StaffList),
        (status = 401, description = "No valid key or session", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "This role does not manage staff", body = Problem, content_type = "application/problem+json"),
    )
)]
pub(super) async fn list_staff(
    State(accounts): State<Accounts>,
    Extension(principal): Extension<Principal>,
) -> Result<Json<StaffList>, Problem> {
    let role = acting_role(&principal).filter(|role| Permission::ManageStaff.granted_to(*role));
    role.ok_or_else(|| forbidden("This role does not manage staff accounts."))?;
    let listed = accounts
        .store()
        .list()
        .await
        .map_err(|error| Problem::internal(&error))?;
    let staff = listed.iter().map(StaffAccount::from).collect();
    Ok(Json(StaffList { staff }))
}

/// Invites a person with a role the caller may manage. They have no
/// password until they choose one.
#[utoipa::path(
    post, path = "/staff/", tag = "staff",
    security(("staff_session" = []), ("admin_key" = [])),
    request_body = Invite,
    responses(
        (status = 201, description = "The new account", body = StaffList),
        (status = 401, description = "No valid key or session", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "This role cannot give that role", body = Problem, content_type = "application/problem+json"),
        (status = 409, description = "An account already has this email", body = Problem, content_type = "application/problem+json"),
        (status = 422, description = "The body could not be read", body = Problem, content_type = "application/problem+json"),
    )
)]
pub(super) async fn invite(
    State(accounts): State<Accounts>,
    Extension(principal): Extension<Principal>,
    body: Result<Json<Invite>, JsonRejection>,
) -> Result<(StatusCode, Json<StaffList>), Problem> {
    let unreadable = || invalid("Send JSON with email, name and role, and nothing else.");
    let Json(body) = body.map_err(|_| unreadable())?;
    let email: Email = body
        .email
        .parse()
        .map_err(|_| invalid("Check the email address."))?;
    let role: Role = body.role.parse().map_err(|_| unreadable())?;
    let name = body.name.trim();
    if name.is_empty() {
        return Err(invalid("Give the person a name."));
    }
    if !acting_role(&principal).is_some_and(|actor| may_manage(actor, role)) {
        return Err(forbidden("This role cannot invite someone with that role."));
    }
    let now = accounts.clock().now();
    match accounts.store().create(&email, name, role, now).await {
        Ok(account) => Ok((StatusCode::CREATED, Json(StaffList::of(&account)))),
        Err(CreateAccountError::Unavailable { source }) => Err(Problem::internal(&source)),
        Err(_) => Err(taken()),
    }
}

pub(super) fn forbidden(what: &'static str) -> Problem {
    let message = UserMessage::new(
        what,
        "Each role manages only the roles below it, so nobody hands out more than they hold.",
        "Ask an owner or an admin to do this.",
    );
    Problem::new(ProblemType::Forbidden, &message)
}

pub(super) fn invalid(fix: &'static str) -> Problem {
    let message = UserMessage::new(
        "The request could not be read.",
        "Khalid needs a valid email, a name and a role to make an account.",
        fix,
    );
    Problem::new(ProblemType::InvalidBody, &message)
}

fn taken() -> Problem {
    let message = UserMessage::new(
        "An account already has this email.",
        "One email holds one account, so sign-in links reach the right person.",
        "Change that account's role instead of inviting it again.",
    );
    Problem::new(ProblemType::Conflict, &message)
}
