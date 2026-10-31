//! The staff routes `khalid-auth` adds to the Admin API under
//! `/api/admin/v1/`: signing in and out, and managing staff (BL-API-4).

mod links;
mod passkey_list;
mod passkeys;
mod staff;
mod staff_changes;
mod take_back;

use std::net::SocketAddr;

use axum::extract::{ConnectInfo, State};
use axum::http::header::{COOKIE, RETRY_AFTER, SET_COOKIE};
use axum::http::{HeaderMap, HeaderValue};
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use khalid_api::problem::{Problem, ProblemType};
use khalid_api::session_check::StaffRoutes;
use khalid_core::UserMessage;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::{Account, Accounts, Password, SessionToken, SignInError, SignedIn};

/// The staff routes for a site's [`Accounts`], for
/// `khalid_api::api::router_with_staff`.
pub fn staff_routes(accounts: Accounts) -> StaffRoutes {
    StaffRoutes {
        open: OpenApiRouter::new()
            .routes(routes!(sign_in))
            .routes(routes!(links::send_link))
            .routes(routes!(links::redeem_link))
            .routes(routes!(passkeys::sign_in_options))
            .routes(routes!(passkeys::passkey_sign_in))
            .with_state(accounts.clone()),
        signed_in: OpenApiRouter::new()
            .routes(routes!(current_session, take_back::sign_out))
            .routes(routes!(take_back::sign_out_everywhere))
            .routes(routes!(take_back::change_password))
            .routes(routes!(staff::list_staff, staff::invite))
            .routes(routes!(passkeys::passkey_options))
            .routes(routes!(passkeys::add_passkey, passkey_list::list_passkeys))
            .routes(routes!(
                staff_changes::change_role,
                staff_changes::remove_staff
            ))
            .routes(routes!(staff_changes::transfer))
            .with_state(accounts),
    }
}

/// A staff account as the Admin API shows it.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct StaffAccount {
    /// The account's number on this site.
    id: i64,
    /// The address they sign in with.
    email: String,
    /// The name the admin shows.
    name: String,
    /// `owner`, `admin`, `editor`, `writer` or `contributor`.
    role: String,
}

impl From<&Account> for StaffAccount {
    fn from(account: &Account) -> Self {
        Self {
            id: account.id().as_row(),
            email: account.email().as_str().to_owned(),
            name: account.name().to_owned(),
            role: account.role().as_str().to_owned(),
        }
    }
}

/// A signed-in session: who it is, and the CSRF token every request with
/// the cookie sends in `x-khalid-csrf`.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SessionAnswer {
    /// Who signed in.
    account: StaffAccount,
    /// The session's CSRF token.
    csrf: String,
}

impl From<&SignedIn> for SessionAnswer {
    fn from(signed_in: &SignedIn) -> Self {
        Self {
            account: StaffAccount::from(&signed_in.account),
            csrf: signed_in.csrf.as_str().to_owned(),
        }
    }
}

/// An email and password to sign in with.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SignInBody {
    /// The address the account signs in with.
    email: String,
    /// The account's password.
    password: String,
}

/// Signs in with an email and password. The answer sets the session
/// cookie and holds the CSRF token. Needs no key.
#[utoipa::path(
    post, path = "/session/", tag = "staff",
    request_body = SignInBody,
    responses(
        (status = 200, description = "Signed in; the cookie is set", body = SessionAnswer),
        (status = 401, description = "The email and password do not match an account", body = Problem, content_type = "application/problem+json"),
        (status = 422, description = "The body could not be read", body = Problem, content_type = "application/problem+json"),
        (status = 429, description = "Too many failed sign-ins; Retry-After says when to try again", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn sign_in(
    State(accounts): State<Accounts>,
    from: Option<Extension<ConnectInfo<SocketAddr>>>,
    body: Result<Json<SignInBody>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let Ok(Json(body)) = body else {
        return invalid_body();
    };
    let (Ok(email), Ok(password)) = (body.email.parse(), Password::new(body.password)) else {
        return refused();
    };
    let from = from.map(|Extension(ConnectInfo(address))| address.ip());
    match accounts.sign_in(&email, password, from).await {
        Ok((token, signed_in)) => {
            let mut response = Json(SessionAnswer::from(&signed_in)).into_response();
            set_cookie(&mut response, token.set_cookie().expose());
            response
        }
        Err(error) => problem(&error),
    }
}

/// Who the session cookie signs in, with its CSRF token again, for an
/// admin page that was reloaded. Needs the session cookie.
#[utoipa::path(
    get, path = "/session/", tag = "staff",
    security(("staff_session" = [])),
    responses(
        (status = 200, description = "The signed-in account", body = SessionAnswer),
        (status = 401, description = "No live session", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn current_session(State(accounts): State<Accounts>, headers: HeaderMap) -> Response {
    let Some(token) = cookie_token(&headers) else {
        return refused();
    };
    match accounts.session(&token).await {
        Ok(signed_in) => Json(SessionAnswer::from(&signed_in)).into_response(),
        Err(error) => problem(&error),
    }
}

fn cookie_token(headers: &HeaderMap) -> Option<SessionToken> {
    let cookies = headers.get(COOKIE)?.to_str().ok()?;
    SessionToken::from_cookie_header(cookies)
}

fn set_cookie(response: &mut Response, value: &str) {
    if let Ok(value) = HeaderValue::from_str(value) {
        response.headers_mut().append(SET_COOKIE, value);
    }
}

/// The answer for a sign-in or session that did not work.
pub(crate) fn problem(error: &SignInError) -> Response {
    match error {
        SignInError::Limited { minutes } => {
            let message = UserMessage::new(
                "There were too many failed sign-ins for this email or from this address.",
                "Waiting between tries stops people guessing passwords.",
                format!("Try again in {minutes} minutes."),
            );
            let mut response = Problem::new(ProblemType::TooManyRequests, &message).into_response();
            let seconds = minutes.saturating_mul(60).to_string();
            if let Ok(value) = HeaderValue::from_str(&seconds) {
                response.headers_mut().insert(RETRY_AFTER, value);
            }
            response
        }
        SignInError::Unavailable { .. } => Problem::internal(error).into_response(),
        _ => refused(),
    }
}

fn refused() -> Response {
    let message = UserMessage::new(
        "The email and password do not match an account, or the session has ended.",
        "The admin can change the site, so it lets in only people who prove who they are.",
        "Check the email and password and sign in again.",
    );
    Problem::new(ProblemType::Unauthorized, &message).into_response()
}

fn invalid_body() -> Response {
    let message = UserMessage::new(
        "The sign-in request could not be read.",
        "Khalid needs an email and a password to sign someone in.",
        "Send JSON with email and password, and nothing else.",
    );
    Problem::new(ProblemType::InvalidBody, &message).into_response()
}
