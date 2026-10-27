//! Signing in by a link sent by email, over the Admin API.

use std::net::SocketAddr;

use axum::extract::rejection::JsonRejection;
use axum::extract::{ConnectInfo, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use khalid_api::problem::{Problem, ProblemType};
use khalid_core::UserMessage;
use serde::Deserialize;
use utoipa::ToSchema;

use super::{SessionAnswer, problem, refused, set_cookie};
use crate::{Accounts, Email, LinkToken, SignInError};

/// The email to send a sign-in link to.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct LinkRequest {
    /// The address the account signs in with.
    email: String,
}

/// The token from a sign-in link, which the admin's sign-in page reads
/// after the `#` and posts here.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct LinkRedeem {
    /// The token.
    token: String,
}

/// Mails a sign-in link to an email that has an account. The answer is
/// the same whether it has one or not. Needs no key.
#[utoipa::path(
    post, path = "/session/link/", tag = "staff",
    request_body = LinkRequest,
    responses(
        (status = 202, description = "If the email has an account, a link is on its way"),
        (status = 404, description = "This site cannot send sign-in links", body = Problem, content_type = "application/problem+json"),
        (status = 422, description = "The body could not be read", body = Problem, content_type = "application/problem+json"),
        (status = 429, description = "Too many links asked for; Retry-After says when to try again", body = Problem, content_type = "application/problem+json"),
    )
)]
pub(super) async fn send_link(
    State(accounts): State<Accounts>,
    from: Option<Extension<ConnectInfo<SocketAddr>>>,
    body: Result<Json<LinkRequest>, JsonRejection>,
) -> Response {
    let Some(email) = body
        .ok()
        .and_then(|Json(body)| body.email.parse::<Email>().ok())
    else {
        return unreadable();
    };
    let from = from.map(|Extension(ConnectInfo(address))| address.ip());
    match accounts.send_link(&email, from).await {
        Ok(()) => StatusCode::ACCEPTED.into_response(),
        Err(SignInError::LinksOff) => links_off(),
        Err(error) => problem(&error),
    }
}

/// Signs in with the token from a sign-in link. The answer sets the
/// session cookie, like signing in with a password. Needs no key.
#[utoipa::path(
    post, path = "/session/link/redeem/", tag = "staff",
    request_body = LinkRedeem,
    responses(
        (status = 200, description = "Signed in; the cookie is set", body = SessionAnswer),
        (status = 401, description = "The link was used, has expired or is unknown", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "This site cannot send sign-in links", body = Problem, content_type = "application/problem+json"),
    )
)]
pub(super) async fn redeem_link(
    State(accounts): State<Accounts>,
    body: Result<Json<LinkRedeem>, JsonRejection>,
) -> Response {
    let Some(token) = body
        .ok()
        .and_then(|Json(body)| LinkToken::parse(&body.token).ok())
    else {
        return refused();
    };
    match accounts.redeem_link(&token).await {
        Ok((session, signed_in)) => {
            let mut response = Json(SessionAnswer::from(&signed_in)).into_response();
            set_cookie(&mut response, session.set_cookie().expose());
            response
        }
        Err(SignInError::LinksOff) => links_off(),
        Err(error) => problem(&error),
    }
}

fn links_off() -> Response {
    let message = UserMessage::new(
        "This site cannot send sign-in links yet.",
        "A link needs mail, and no mail sender is set up.",
        "Sign in with your password or a passkey, or ask the owner to set up mail.",
    );
    Problem::new(ProblemType::NotFound, &message).into_response()
}

fn unreadable() -> Response {
    let message = UserMessage::new(
        "The request for a sign-in link could not be read.",
        "Khalid needs an email address to send the link to.",
        "Send JSON with email, and nothing else.",
    );
    Problem::new(ProblemType::InvalidBody, &message).into_response()
}
