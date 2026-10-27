//! Signing out, signing out everywhere and changing the password over the
//! Admin API.

use axum::Json;
use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use khalid_api::problem::{Problem, ProblemType};
use khalid_core::UserMessage;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{cookie_token, problem, refused, set_cookie};
use crate::{Accounts, CLEAR_COOKIE, Password, WeakPasswordError};

/// Signs out: ends this session and clears the cookie.
#[utoipa::path(
    delete, path = "/session/", tag = "staff",
    security(("staff_session" = [])),
    responses(
        (status = 204, description = "Signed out; the cookie is cleared"),
        (status = 401, description = "No live session", body = Problem, content_type = "application/problem+json"),
    )
)]
pub(super) async fn sign_out(State(accounts): State<Accounts>, headers: HeaderMap) -> Response {
    let Some(token) = cookie_token(&headers) else {
        return refused();
    };
    if let Err(error) = accounts.sign_out(&token).await {
        return problem(&error);
    }
    let mut response = StatusCode::NO_CONTENT.into_response();
    set_cookie(&mut response, CLEAR_COOKIE);
    response
}

/// How many sessions signing out everywhere ended.
#[derive(Debug, Serialize, ToSchema)]
pub(super) struct Ended {
    /// Sessions ended, this one included.
    ended: u64,
}

/// Signs out everywhere: ends every session of this account, this one
/// too, for a lost laptop or a stolen cookie.
#[utoipa::path(
    delete, path = "/sessions/", tag = "staff",
    security(("staff_session" = [])),
    responses(
        (status = 200, description = "Every session ended; the cookie is cleared", body = Ended),
        (status = 401, description = "No live session", body = Problem, content_type = "application/problem+json"),
    )
)]
pub(super) async fn sign_out_everywhere(
    State(accounts): State<Accounts>,
    headers: HeaderMap,
) -> Response {
    let Some(token) = cookie_token(&headers) else {
        return refused();
    };
    match accounts.sign_out_everywhere(&token).await {
        Ok(ended) => {
            let mut response = Json(Ended { ended }).into_response();
            set_cookie(&mut response, CLEAR_COOKIE);
            response
        }
        Err(error) => problem(&error),
    }
}

/// A new password, with the current one when the account has one.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct PasswordChange {
    /// The password now; leave it out only to set a first password.
    current: Option<String>,
    /// The new password: at least 12 characters.
    new: String,
}

/// Changes the password of the signed-in account and ends its other
/// sessions. An account with no password may set its first one only in
/// the 15 minutes after signing in.
#[utoipa::path(
    put, path = "/session/password/", tag = "staff",
    security(("staff_session" = [])),
    request_body = PasswordChange,
    responses(
        (status = 204, description = "Changed; other sessions ended"),
        (status = 401, description = "No live session, or the current password is wrong", body = Problem, content_type = "application/problem+json"),
        (status = 422, description = "The new password is too short or too long", body = Problem, content_type = "application/problem+json"),
    )
)]
pub(super) async fn change_password(
    State(accounts): State<Accounts>,
    headers: HeaderMap,
    body: Result<Json<PasswordChange>, JsonRejection>,
) -> Response {
    let Some(token) = cookie_token(&headers) else {
        return refused();
    };
    let Ok(Json(body)) = body else {
        return weak(None);
    };
    let new = match Password::new(body.new) {
        Ok(new) => new,
        Err(error) => return weak(Some(error)),
    };
    let current = body.current.and_then(|text| Password::new(text).ok());
    match accounts.change_password(&token, current, new).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => problem(&error),
    }
}

fn weak(error: Option<WeakPasswordError>) -> Response {
    let what = match error {
        Some(WeakPasswordError::TooLong) => "The new password is longer than 1024 bytes.",
        Some(_) => "The new password has fewer than 12 characters.",
        None => "The password change could not be read.",
    };
    let message = UserMessage::new(
        what,
        "A long password is what makes it hard to guess.",
        "Send JSON with new, at least 12 characters, and current if the account has a password.",
    );
    Problem::new(ProblemType::InvalidBody, &message).into_response()
}
