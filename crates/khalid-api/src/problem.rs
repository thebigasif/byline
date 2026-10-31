//! RFC 9457 problem details, the shape every API error takes.

use axum::Json;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use khalid_core::UserMessage;
use khalid_lint::Report;
use serde::{Serialize, Serializer};
use utoipa::ToSchema;

use crate::query::{ParseQueryError, QueryErrorKind};

/// Where problem types are documented.
const TYPE_BASE: &str = "https://khalid.dev/problems/";

/// The kinds of problem the API reports, each with its status and title.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProblemType {
    /// A list parameter could not be read.
    InvalidQuery,
    /// Nothing is at the address.
    NotFound,
    /// The linter refused to publish.
    LintBlocked,
    /// The server failed while answering.
    Internal,
    /// The request carried no key the API accepts.
    Unauthorized,
    /// The key is known but may not do this.
    Forbidden,
    /// The request body could not be read.
    InvalidBody,
    /// Someone else changed the resource first.
    Conflict,
    /// `Accept-Version` is not a version.
    InvalidVersion,
    /// `Accept-Version` asks for a version this server does not serve.
    VersionNotSupported,
    /// Too many attempts in a short time; the answer says when to retry.
    TooManyRequests,
    /// A feature is turned off or lacks a key, or a service it relies on
    /// did not answer.
    Unavailable,
}

impl ProblemType {
    fn slug(self) -> &'static str {
        match self {
            Self::InvalidQuery => "invalid-query",
            Self::NotFound => "not-found",
            Self::LintBlocked => "lint-blocked",
            Self::Internal => "internal",
            Self::Unauthorized => "unauthorized",
            Self::Forbidden => "forbidden",
            Self::InvalidBody => "invalid-body",
            Self::Conflict => "conflict",
            Self::InvalidVersion => "invalid-version",
            Self::VersionNotSupported => "version-not-supported",
            Self::TooManyRequests => "too-many-requests",
            Self::Unavailable => "unavailable",
        }
    }

    fn status(self) -> StatusCode {
        match self {
            Self::InvalidQuery | Self::InvalidVersion => StatusCode::BAD_REQUEST,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::LintBlocked | Self::InvalidBody => StatusCode::UNPROCESSABLE_ENTITY,
            Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::Conflict => StatusCode::CONFLICT,
            Self::VersionNotSupported => StatusCode::NOT_ACCEPTABLE,
            Self::TooManyRequests => StatusCode::TOO_MANY_REQUESTS,
            Self::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::InvalidQuery => "The list could not be read",
            Self::NotFound => "Nothing is at this address",
            Self::LintBlocked => "The post cannot be published",
            Self::Internal => "The server could not answer",
            Self::Unauthorized => "The key was not accepted",
            Self::Forbidden => "This key cannot do that",
            Self::InvalidBody => "The request could not be read",
            Self::Conflict => "Someone else changed this first",
            Self::InvalidVersion => "The version could not be read",
            Self::VersionNotSupported => "This version is not served here",
            Self::TooManyRequests => "Too many attempts",
            Self::Unavailable => "This is not available now",
        }
    }
}

/// An error as the API sends it (RULES.md BL-ERR-5): `type`, `title`,
/// `status` and `detail`, plus `fix`, and `rule` and `instance` where they
/// apply. `detail` and `fix` come from a [`UserMessage`], so they say what
/// happened, why it matters and what to do (BL-ERR-4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct Problem {
    /// A link to the page that explains this kind of problem.
    #[serde(rename = "type", serialize_with = "type_link")]
    #[schema(value_type = String, example = "https://khalid.dev/problems/not-found")]
    kind: ProblemType,
    /// The same short title for every problem of this kind.
    title: &'static str,
    /// The HTTP status.
    status: u16,
    /// What happened and why it matters.
    detail: String,
    /// What to do.
    fix: String,
    /// The lint rule or parameter that produced the problem.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>)]
    rule: Option<Box<str>>,
    /// The address the problem is about.
    #[serde(skip_serializing_if = "Option::is_none")]
    instance: Option<String>,
    /// For a post the linter refused, every finding, as `khalid lint
    /// --json` prints them for one post, and the post's `score` out of 100
    /// with the rules that cost it points.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>)]
    lint: Option<Box<Report>>,
}

impl Problem {
    /// A problem of `kind`, explained by `message`.
    pub fn new(kind: ProblemType, message: &UserMessage) -> Self {
        Self {
            kind,
            title: kind.title(),
            status: kind.status().as_u16(),
            detail: format!("{} {}", message.what(), message.why()),
            fix: message.fix().to_owned(),
            rule: None,
            instance: None,
            lint: None,
        }
    }

    /// A 500 for a failure inside the server: `error` and its causes go to
    /// the log through `tracing`, and the person reads only what to do.
    pub fn internal(error: &(dyn std::error::Error + 'static)) -> Self {
        tracing::error!(error, "the API could not answer");
        let message = UserMessage::new(
            "The server could not read what it needed.",
            "Nothing was changed, but this request could not be answered.",
            "Try again in a minute, and check the server log if it keeps happening.",
        );
        Self::new(ProblemType::Internal, &message)
    }

    /// The same problem, naming the rule or parameter behind it.
    #[must_use]
    pub fn with_rule(self, rule: &str) -> Self {
        Self {
            rule: Some(rule.into()),
            ..self
        }
    }

    /// The same problem, carrying the linter's `report`.
    #[must_use]
    pub fn with_lint(self, report: Report) -> Self {
        Self {
            lint: Some(Box::new(report)),
            ..self
        }
    }

    /// The same problem, naming the address it is about.
    #[must_use]
    pub fn with_instance(self, instance: &str) -> Self {
        Self {
            instance: Some(instance.to_owned()),
            ..self
        }
    }
}

impl IntoResponse for Problem {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let media = HeaderValue::from_static("application/problem+json");
        (status, [(header::CONTENT_TYPE, media)], Json(self)).into_response()
    }
}

impl From<ParseQueryError> for Problem {
    fn from(error: ParseQueryError) -> Self {
        let fix = match error.kind() {
            QueryErrorKind::Filter(_) => {
                "Write each condition as field:value, for example tag:news."
            }
            QueryErrorKind::Limit => "Use a number from 1 to 100, or all.",
            QueryErrorKind::Page => "Use a page number from 1 upwards.",
            QueryErrorKind::UnknownFormat(_) => "Use html, markdown or plaintext.",
            QueryErrorKind::BadName(_) => {
                "Use lowercase field names such as title or published_at."
            }
            QueryErrorKind::EmptyName => "Remove the extra comma.",
            QueryErrorKind::BadDirection(_) => "Write asc or desc after the field name.",
            QueryErrorKind::BadOrderKey(_) => {
                "Write each key as a field and a direction, such as published_at desc."
            }
            QueryErrorKind::TooMany(_) => "Ask for fewer names in one request.",
        };
        let message = UserMessage::new(
            sentence(&error.to_string()),
            "The list cannot be sent until it is fixed.",
            fix,
        );
        Self::new(ProblemType::InvalidQuery, &message).with_rule(error.parameter())
    }
}

/// Writes the link to the page that explains `kind`.
#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde's serialize_with passes the field by reference"
)]
fn type_link<S: Serializer>(kind: &ProblemType, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_str(&format_args!("{TYPE_BASE}{}", kind.slug()))
}

/// `text` with a capital first letter and a full stop.
fn sentence(text: &str) -> String {
    let mut chars = text.chars();
    let first = chars
        .next()
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_default();
    format!("{first}{}.", chars.as_str())
}
