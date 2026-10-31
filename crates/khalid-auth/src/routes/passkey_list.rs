//! A signed-in person's own passkeys over the Admin API: the list, a new
//! name for one, and removing one.

use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use khalid_api::problem::Problem;
use serde::Serialize;
use utoipa::ToSchema;

use super::passkeys::passkey_problem;
use super::{cookie_token, refused};
use crate::{Accounts, ListedPasskey};

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
