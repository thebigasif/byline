//! `Accept-Version` and `Content-Version` (`docs/standards/api.md`).

use axum::extract::Request;
use axum::http::HeaderValue;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use khalid_core::UserMessage;
use utoipa::openapi::path::{Operation, Parameter, ParameterBuilder, ParameterIn};
use utoipa::openapi::schema::{ObjectBuilder, Type};
use utoipa::openapi::{OpenApi, Required};

use crate::problem::{Problem, ProblemType};

/// The version this server serves, as major and minor.
const SERVED: (u32, u32) = (1, 0);
/// The same, as the `Content-Version` header writes it.
const SERVED_TEXT: &str = "v1.0";

/// Serves a request whose `Accept-Version` this server meets, and names
/// the version served on every answer. `Accept-Version: v1.3` asks for at
/// least 1.3, so a server at 1.0 refuses it rather than leave out what the
/// client counts on.
pub(crate) async fn negotiate_version(request: Request, next: Next) -> Response {
    let asked = request
        .headers()
        .get("accept-version")
        .map(|value| value.to_str().ok().and_then(parse));
    let mut response = match asked {
        None => next.run(request).await,
        Some(Some((major, minor))) if major == SERVED.0 && minor <= SERVED.1 => {
            next.run(request).await
        }
        Some(Some(_)) => refuse(ProblemType::VersionNotSupported),
        Some(None) => refuse(ProblemType::InvalidVersion),
    };
    let served = HeaderValue::from_static(SERVED_TEXT);
    response.headers_mut().insert("content-version", served);
    response
}

/// `v1` or `v1.3`, with either case of `v`.
fn parse(text: &str) -> Option<(u32, u32)> {
    let number = text.strip_prefix(['v', 'V'])?;
    let (major, minor) = number.split_once('.').unwrap_or((number, "0"));
    let digits = |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
    if !digits(major) || !digits(minor) {
        return None;
    }
    Some((major.parse().ok()?, minor.parse().ok()?))
}

fn refuse(kind: ProblemType) -> Response {
    let message = match kind {
        ProblemType::InvalidVersion => UserMessage::new(
            "The Accept-Version header is not a version.",
            "The server cannot tell which version the client expects.",
            "Send a version such as v1.0, or leave the header out.",
        ),
        _ => UserMessage::new(
            "This server serves version v1.0 of the API, older than the client asked for.",
            "Answering anyway could leave out fields the client counts on.",
            "Update Khalid on the server, or ask for v1.0.",
        ),
    };
    Problem::new(kind, &message)
        .with_rule("accept-version")
        .into_response()
}

/// Lists the optional `Accept-Version` header on every operation in
/// `document` (BL-API-4).
pub(crate) fn document(document: &mut OpenApi) {
    let header: Parameter = ParameterBuilder::new()
        .name("Accept-Version")
        .parameter_in(ParameterIn::Header)
        .required(Required::False)
        .description(Some(
            "The oldest API version the client accepts, such as v1.0.",
        ))
        .schema(Some(ObjectBuilder::new().schema_type(Type::String)))
        .build();
    for item in document.paths.paths.values_mut() {
        let operations: [&mut Option<Operation>; 5] = [
            &mut item.get,
            &mut item.put,
            &mut item.post,
            &mut item.patch,
            &mut item.delete,
        ];
        for operation in operations.into_iter().flatten() {
            let parameters = operation.parameters.get_or_insert_with(Vec::new);
            parameters.push(header.clone().into());
        }
    }
}
