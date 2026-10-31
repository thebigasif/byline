//! A signed-in person's own passkeys over the Admin API: the list, a new
//! name for one, and removing one.

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use khalid_api::problem::{Problem, ProblemType};
use khalid_core::UserMessage;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::passkeys::passkey_problem;
use super::{cookie_token, refused};
use crate::{Accounts, ListedPasskey, MAX_NAME_CHARS, PasskeyId, PasskeyName, SignInError};

/// One of the signed-in person's passkeys. The key itself never leaves
/// the server.
#[derive(Debug, Serialize, ToSchema)]
pub(super) struct PasskeyEntry {
    /// The passkey's number, for renaming or removing it.
    id: i64,
    /// The name the person gave it, such as "Work laptop".
    name: String,
    /// When it was added.
    added_at: DateTime<Utc>,
    /// When it last signed someone in; `null` when it never has.
    last_used_at: Option<DateTime<Utc>>,
}

impl From<ListedPasskey> for PasskeyEntry {
    fn from(listed: ListedPasskey) -> Self {
        Self {
            id: listed.id.as_row(),
            name: listed.name,
            added_at: listed.added_at,
            last_used_at: listed.last_used_at,
        }
    }
}

/// A new name for a passkey.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct PasskeyRename {
    /// The new name, 1 to 60 characters once trimmed.
    name: String,
}

/// The signed-in person's passkeys, oldest first. Needs the session
/// cookie.
#[utoipa::path(
    get, path = "/session/passkeys/", tag = "staff",
    security(("staff_session" = [])),
    responses(
        (status = 200, description = "The passkeys of the signed-in account", body = Vec<PasskeyEntry>),
        (status = 401, description = "No live session", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "This site has no passkeys", body = Problem, content_type = "application/problem+json"),
    )
)]
pub(super) async fn list_passkeys(
    State(accounts): State<Accounts>,
    headers: HeaderMap,
) -> Response {
    let Some(token) = cookie_token(&headers) else {
        return refused();
    };
    match accounts.passkeys_of(&token).await {
        Ok(listed) => {
            let entries: Vec<PasskeyEntry> = listed.into_iter().map(PasskeyEntry::from).collect();
            Json(entries).into_response()
        }
        Err(error) => passkey_problem(&error),
    }
}

/// Gives one of the signed-in person's passkeys a new name. Needs the
/// session cookie.
#[utoipa::path(
    patch, path = "/session/passkeys/{id}/", tag = "staff",
    security(("staff_session" = [])),
    params(("id" = i64, Path, description = "The passkey's number")),
    request_body = PasskeyRename,
    responses(
        (status = 204, description = "Renamed"),
        (status = 401, description = "No live session", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "The account has no passkey with this number", body = Problem, content_type = "application/problem+json"),
        (status = 422, description = "The name is blank or too long", body = Problem, content_type = "application/problem+json"),
    )
)]
pub(super) async fn rename_passkey(
    State(accounts): State<Accounts>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<PasskeyRename>, JsonRejection>,
) -> Response {
    let Some(token) = cookie_token(&headers) else {
        return refused();
    };
    let Ok(Json(body)) = body else {
        return bad_name();
    };
    let Ok(name) = PasskeyName::new(&body.name) else {
        return bad_name();
    };
    let Some(id) = number(&id) else {
        return passkey_problem(&SignInError::NoPasskey);
    };
    match accounts.rename_passkey(&token, id, &name).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => passkey_problem(&error),
    }
}

/// Removes one of the signed-in person's passkeys, so it signs nobody in
/// again. Needs the session cookie.
#[utoipa::path(
    delete, path = "/session/passkeys/{id}/", tag = "staff",
    security(("staff_session" = [])),
    params(("id" = i64, Path, description = "The passkey's number")),
    responses(
        (status = 204, description = "Removed"),
        (status = 401, description = "No live session", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "The account has no passkey with this number", body = Problem, content_type = "application/problem+json"),
    )
)]
pub(super) async fn remove_passkey(
    State(accounts): State<Accounts>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let Some(token) = cookie_token(&headers) else {
        return refused();
    };
    let Some(id) = number(&id) else {
        return passkey_problem(&SignInError::NoPasskey);
    };
    match accounts.remove_passkey(&token, id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => passkey_problem(&error),
    }
}

/// The passkey numbered `id`; `None` for text that is not a number.
fn number(id: &str) -> Option<PasskeyId> {
    id.parse().ok().map(PasskeyId::from_row)
}

/// The answer for a passkey name that cannot be used.
pub(super) fn bad_name() -> Response {
    let message = UserMessage::new(
        format!("A passkey's name needs 1 to {MAX_NAME_CHARS} characters."),
        "The name tells your passkeys apart in this list.",
        "Send JSON with name, such as \"Work laptop\".",
    );
    Problem::new(ProblemType::InvalidBody, &message).into_response()
}
