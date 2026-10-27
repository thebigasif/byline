//! Adding a passkey and signing in with one, over the Admin API. Bytes
//! travel as unpadded base64url, the way browsers give them.

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
use crate::{Accounts, CEREMONY, SignInError};

/// What the admin passes to `navigator.credentials.create`.
#[derive(Debug, Serialize, ToSchema)]
pub(super) struct CreateOptions {
    /// The challenge to sign.
    challenge: String,
    /// The site's host name.
    rp_id: String,
    /// The account's handle for authenticators.
    user_id: String,
    /// Passkeys the account already has.
    exclude: Vec<String>,
    /// The key algorithms Khalid accepts, by their `WebAuthn` numbers:
    /// ES256, then `EdDSA`.
    algorithms: Vec<i64>,
    /// How long the browser may wait, in milliseconds.
    timeout_ms: i64,
}

/// What the admin passes to `navigator.credentials.get`.
#[derive(Debug, Serialize, ToSchema)]
pub(super) struct GetOptions {
    /// The challenge to sign.
    challenge: String,
    /// The site's host name.
    rp_id: String,
    /// How long the browser may wait, in milliseconds.
    timeout_ms: i64,
}

/// A new passkey, as `navigator.credentials.create` returned it.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct NewPasskey {
    /// A name to tell it apart, such as "Work laptop".
    name: String,
    /// `clientDataJSON`.
    client_data: String,
    /// `attestationObject`.
    attestation_object: String,
}

/// A sign-in, as `navigator.credentials.get` returned it.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct PasskeySignIn {
    /// The credential's `rawId`.
    credential_id: String,
    /// `clientDataJSON`.
    client_data: String,
    /// `authenticatorData`.
    authenticator_data: String,
    /// `signature`.
    signature: String,
}

fn decode(text: &str) -> Option<Vec<u8>> {
    URL_SAFE_NO_PAD.decode(text).ok()
}

fn encode(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
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
        Ok(request) => Json(CreateOptions {
            challenge: request.challenge.to_base64(),
            rp_id: request.rp_id,
            user_id: encode(&request.user_id),
            exclude: request.exclude.iter().map(|id| encode(id)).collect(),
            algorithms: vec![-7, -8],
            timeout_ms: CEREMONY.num_milliseconds(),
        })
        .into_response(),
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
    let (Some(client), Some(object)) =
        (decode(&body.client_data), decode(&body.attestation_object))
    else {
        return unreadable();
    };
    let name = body.name.trim();
    match accounts
        .finish_registration(&token, (&client, &object), name)
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
        Ok((challenge, rp_id)) => Json(GetOptions {
            challenge: challenge.to_base64(),
            rp_id,
            timeout_ms: CEREMONY.num_milliseconds(),
        })
        .into_response(),
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
    let id = decode(&body.credential_id);
    let (client, data) = (decode(&body.client_data), decode(&body.authenticator_data));
    let (Some(id), Some(client), Some(data), Some(signature)) =
        (id, client, data, decode(&body.signature))
    else {
        return refused();
    };
    match accounts
        .finish_sign_in(&id, (&client, &data, &signature))
        .await
    {
        Ok((session, signed_in)) => {
            let mut response = Json(SessionAnswer::from(&signed_in)).into_response();
            set_cookie(&mut response, session.set_cookie().expose());
            response
        }
        Err(error) => passkey_problem(&error),
    }
}

fn passkey_problem(error: &SignInError) -> Response {
    match error {
        SignInError::PasskeysOff => {
            let message = UserMessage::new(
                "This site has no passkeys set up.",
                "A passkey needs the site's address, and it is not set.",
                "Sign in with your password or an email link, or ask the owner to set the address.",
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
        "Send JSON with name, client_data and attestation_object in base64url.",
    );
    Problem::new(ProblemType::InvalidBody, &message).into_response()
}
