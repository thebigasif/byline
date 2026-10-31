//! Adding a passkey and signing in with one, over the Admin API, in the
//! standard `WebAuthn` JSON: the options are what the browser's
//! `PublicKeyCredential.parseCreationOptionsFromJSON` and
//! `parseRequestOptionsFromJSON` read, and each answer is the credential's
//! `toJSON()` with the challenge it answers.

use axum::Json;
use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use khalid_api::problem::{Problem, ProblemType};
use khalid_core::UserMessage;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{SessionAnswer, cookie_token, problem, refused, set_cookie};
use crate::{
    Accounts, CreationChallengeResponse, PublicKeyCredential, RegisterPublicKeyCredential,
    RequestChallengeResponse, SignInError,
};

/// What the admin passes to `navigator.credentials.create`, as
/// `{"publicKey": {...}}`.
#[derive(Debug, Serialize, ToSchema)]
pub(super) struct CreateOptions {
    /// The options, in `publicKey`.
    #[serde(flatten)]
    #[schema(value_type = Object)]
    options: CreationChallengeResponse,
}

/// What the admin passes to `navigator.credentials.get`, as
/// `{"publicKey": {...}, "mediation": "conditional"}`.
#[derive(Debug, Serialize, ToSchema)]
pub(super) struct GetOptions {
    /// The options, in `publicKey`, and how the browser should ask.
    #[serde(flatten)]
    #[schema(value_type = Object)]
    options: RequestChallengeResponse,
}

/// A new passkey, as `navigator.credentials.create` returned it.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct NewPasskey {
    /// A name to tell it apart, such as "Work laptop".
    name: String,
    /// The `challenge` of the options it answers, as they gave it.
    challenge: String,
    /// The credential's `toJSON()`.
    #[schema(value_type = Object)]
    credential: RegisterPublicKeyCredential,
}

/// A sign-in, as `navigator.credentials.get` returned it.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct PasskeySignIn {
    /// The `challenge` of the options it answers, as they gave it.
    challenge: String,
    /// The credential's `toJSON()`.
    #[schema(value_type = Object)]
    credential: PublicKeyCredential,
}

fn decode(text: &str) -> Option<Vec<u8>> {
    URL_SAFE_NO_PAD.decode(text).ok()
}

/// Starts adding a passkey to the signed-in account. Needs the session
/// cookie.
#[utoipa::path(
    post, path = "/session/passkeys/options/", tag = "staff",
    security(("staff_session" = [])),
    responses(
        (status = 200, description = "What to pass to navigator.credentials.create", body = CreateOptions),
        (status = 401, description = "No live session", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "This site has no passkeys", body = Problem, content_type = "application/problem+json"),
    )
)]
pub(super) async fn passkey_options(
    State(accounts): State<Accounts>,
    headers: HeaderMap,
) -> Response {
    let Some(token) = cookie_token(&headers) else {
        return refused();
    };
    match accounts.begin_registration(&token).await {
        Ok(options) => Json(CreateOptions { options }).into_response(),
        Err(error) => passkey_problem(&error),
    }
}

/// Adds the passkey the browser made to the signed-in account. Needs the
/// session cookie.
#[utoipa::path(
    post, path = "/session/passkeys/", tag = "staff",
    security(("staff_session" = [])),
    request_body = NewPasskey,
    responses(
        (status = 201, description = "The passkey is added"),
        (status = 401, description = "No live session, or the request was used or too old", body = Problem, content_type = "application/problem+json"),
        (status = 422, description = "The passkey's answer was refused; detail says why", body = Problem, content_type = "application/problem+json"),
    )
)]
pub(super) async fn add_passkey(
    State(accounts): State<Accounts>,
    headers: HeaderMap,
    body: Result<Json<NewPasskey>, JsonRejection>,
) -> Response {
    let Some(token) = cookie_token(&headers) else {
        return refused();
    };
    let Ok(Json(body)) = body else {
        return unreadable();
    };
    let Some(challenge) = decode(&body.challenge) else {
        return unreadable();
    };
    let name = body.name.trim();
    match accounts
        .finish_registration(&token, &challenge, &body.credential, name)
        .await
    {
        Ok(()) => StatusCode::CREATED.into_response(),
        Err(error) => passkey_problem(&error),
    }
}

/// Starts a passkey sign-in. Needs no key.
#[utoipa::path(
    post, path = "/session/passkey/options/", tag = "staff",
    responses(
        (status = 200, description = "What to pass to navigator.credentials.get", body = GetOptions),
        (status = 404, description = "This site has no passkeys", body = Problem, content_type = "application/problem+json"),
    )
)]
pub(super) async fn sign_in_options(State(accounts): State<Accounts>) -> Response {
    match accounts.begin_sign_in() {
        Ok(options) => Json(GetOptions { options }).into_response(),
        Err(error) => passkey_problem(&error),
    }
}

/// Signs in with a passkey. The answer sets the session cookie. Needs no
/// key.
#[utoipa::path(
    post, path = "/session/passkey/", tag = "staff",
    request_body = PasskeySignIn,
    responses(
        (status = 200, description = "Signed in; the cookie is set", body = SessionAnswer),
        (status = 401, description = "The passkey was not accepted", body = Problem, content_type = "application/problem+json"),
    )
)]
pub(super) async fn passkey_sign_in(
    State(accounts): State<Accounts>,
    body: Result<Json<PasskeySignIn>, JsonRejection>,
) -> Response {
    let Ok(Json(body)) = body else {
        return refused();
    };
    let Some(challenge) = decode(&body.challenge) else {
        return refused();
    };
    match accounts.finish_sign_in(&challenge, &body.credential).await {
        Ok((session, signed_in)) => {
            let mut response = Json(SessionAnswer::from(&signed_in)).into_response();
            set_cookie(&mut response, session.set_cookie().expose());
            response
        }
        Err(error) => passkey_problem(&error),
    }
}

pub(super) fn passkey_problem(error: &SignInError) -> Response {
    match error {
        SignInError::PasskeysOff => {
            let message = UserMessage::new(
                "This site has no passkeys set up.",
                "A passkey belongs to the site's host name, and the site address has none, or is not set.",
                "Sign in with your password or an email link, or ask the owner to set a named address.",
            );
            Problem::new(ProblemType::NotFound, &message).into_response()
        }
        SignInError::Passkey(reason) => {
            let message = UserMessage::new(
                format!("The passkey could not be added: {reason}."),
                "Khalid adds only passkeys made for this site, on this page, and unlocked by you.",
                "Try again from the admin's own page, and unlock the passkey when asked.",
            );
            Problem::new(ProblemType::InvalidBody, &message).into_response()
        }
        _ => problem(error),
    }
}

fn unreadable() -> Response {
    let message = UserMessage::new(
        "The new passkey could not be read.",
        "Khalid needs the browser's answer to check the passkey.",
        "Send JSON with name, challenge and the credential's toJSON() as credential.",
    );
    Problem::new(ProblemType::InvalidBody, &message).into_response()
}
