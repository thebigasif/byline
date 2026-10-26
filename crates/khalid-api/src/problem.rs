//! RFC 9457 problem details, the shape every API error takes.

use axum::Json;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use khalid_core::UserMessage;
use serde::Serialize;
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
}

impl ProblemType {
    fn slug(self) -> &'static str {
        match self {
            Self::InvalidQuery => "invalid-query",
            Self::NotFound => "not-found",
            Self::LintBlocked => "lint-blocked",
        }
    }

    fn status(self) -> StatusCode {
        match self {
            Self::InvalidQuery => StatusCode::BAD_REQUEST,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::LintBlocked => StatusCode::UNPROCESSABLE_ENTITY,
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::InvalidQuery => "The list could not be read",
            Self::NotFound => "Nothing is at this address",
            Self::LintBlocked => "The post cannot be published",
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
    #[serde(rename = "type")]
    kind: String,
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
    rule: Option<String>,
    /// The address the problem is about.
    #[serde(skip_serializing_if = "Option::is_none")]
    instance: Option<String>,
}

impl Problem {
    /// A problem of `kind`, explained by `message`.
    pub fn new(kind: ProblemType, message: &UserMessage) -> Self {
        Self {
            kind: format!("{TYPE_BASE}{}", kind.slug()),
            title: kind.title(),
            status: kind.status().as_u16(),
            detail: format!("{} {}", message.what(), message.why()),
            fix: message.fix().to_owned(),
            rule: None,
            instance: None,
        }
    }

    /// The same problem, naming the rule or parameter behind it.
    #[must_use]
    pub fn with_rule(self, rule: &str) -> Self {
        Self {
            rule: Some(rule.to_owned()),
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

/// `text` with a capital first letter and a full stop.
fn sentence(text: &str) -> String {
    let mut chars = text.chars();
    let first = chars
        .next()
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_default();
    format!("{first}{}.", chars.as_str())
}
