//! The list parameters, read and checked together.

use std::error::Error;
use std::fmt;

use serde::Deserialize;

use super::{Filter, ParseFilterError};

/// Items per page when `limit` is not given, as in Ghost.
const DEFAULT_LIMIT: u32 = 15;
/// The largest `limit` other than `all`.
const MAX_LIMIT: u32 = 100;
/// The highest page number accepted.
const MAX_PAGE: u32 = 1_000_000;

/// The list parameters as they arrive in the query string, before any
/// checking. Other parameters, such as `key`, are left to the caller.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[non_exhaustive]
pub struct RawListQuery {
    /// `filter=tag:news+featured:true`
    pub filter: Option<String>,
    /// `formats=html,plaintext`
    pub formats: Option<String>,
    /// `limit=15` or `limit=all`
    pub limit: Option<String>,
    /// `page=2`
    pub page: Option<String>,
}

/// The list parameters, checked (RULES.md BL-API-5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListQuery {
    filter: Option<Filter>,
    formats: Vec<Format>,
    limit: Limit,
    page: u32,
}

impl ListQuery {
    /// The filter, if one was given.
    pub fn filter(&self) -> Option<&Filter> {
        self.filter.as_ref()
    }
    /// Body formats to return, in the order asked, without repeats; `html`
    /// unless asked otherwise.
    pub fn formats(&self) -> &[Format] {
        &self.formats
    }
    /// Items per page.
    pub fn limit(&self) -> Limit {
        self.limit
    }
    /// The page, counted from one.
    pub fn page(&self) -> u32 {
        self.page
    }
}

impl TryFrom<&RawListQuery> for ListQuery {
    type Error = ParseQueryError;

    fn try_from(raw: &RawListQuery) -> Result<Self, Self::Error> {
        let filter = raw.filter.as_deref().map(str::parse).transpose();
        Ok(Self {
            filter: filter
                .map_err(|error| ParseQueryError::new("filter", QueryErrorKind::Filter(error)))?,
            formats: raw
                .formats
                .as_deref()
                .map_or(Ok(vec![Format::Html]), formats)?,
            limit: raw
                .limit
                .as_deref()
                .map_or(Ok(Limit::Count(DEFAULT_LIMIT)), limit)?,
            page: raw.page.as_deref().map_or(Ok(1), page)?,
        })
    }
}

/// How many items a page holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Limit {
    /// At most this many, from 1 to 100.
    Count(u32),
    /// Everything, on one page.
    All,
}

/// A body format a list can return.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Format {
    /// Rendered HTML.
    Html,
    /// The markdown source.
    Markdown,
    /// Text with the markup removed.
    Plaintext,
}

fn formats(text: &str) -> Result<Vec<Format>, ParseQueryError> {
    let mut found = Vec::new();
    for name in text.split(',').map(str::trim) {
        let format = match name {
            "html" => Format::Html,
            "markdown" => Format::Markdown,
            "plaintext" => Format::Plaintext,
            _ => {
                let kind = QueryErrorKind::UnknownFormat(name.to_owned());
                return Err(ParseQueryError::new("formats", kind));
            }
        };
        if !found.contains(&format) {
            found.push(format);
        }
    }
    Ok(found)
}

fn limit(text: &str) -> Result<Limit, ParseQueryError> {
    if text == "all" {
        return Ok(Limit::All);
    }
    text.parse()
        .ok()
        .filter(|count| (1..=MAX_LIMIT).contains(count))
        .map(Limit::Count)
        .ok_or_else(|| ParseQueryError::new("limit", QueryErrorKind::Limit))
}

fn page(text: &str) -> Result<u32, ParseQueryError> {
    text.parse()
        .ok()
        .filter(|page| (1..=MAX_PAGE).contains(page))
        .ok_or_else(|| ParseQueryError::new("page", QueryErrorKind::Page))
}

/// A list parameter that could not be read, naming the parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseQueryError {
    parameter: &'static str,
    kind: QueryErrorKind,
}

impl ParseQueryError {
    fn new(parameter: &'static str, kind: QueryErrorKind) -> Self {
        Self { parameter, kind }
    }
    /// The parameter, such as `limit`.
    pub fn parameter(&self) -> &'static str {
        self.parameter
    }
    /// What was wrong.
    pub fn kind(&self) -> &QueryErrorKind {
        &self.kind
    }
}

/// What was wrong with a list parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum QueryErrorKind {
    /// The filter could not be read.
    Filter(ParseFilterError),
    /// `limit` is not 1 to 100 or `all`.
    Limit,
    /// `page` is not 1 to 1,000,000.
    Page,
    /// A format other than html, markdown or plaintext.
    UnknownFormat(String),
}

impl fmt::Display for ParseQueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            QueryErrorKind::Filter(error) => write!(f, "{error}"),
            QueryErrorKind::Limit => write!(
                f,
                "the limit parameter must be a whole number from 1 to {MAX_LIMIT}, or all"
            ),
            QueryErrorKind::Page => write!(
                f,
                "the page parameter must be a whole number from 1 to {MAX_PAGE}"
            ),
            QueryErrorKind::UnknownFormat(name) => write!(
                f,
                "the formats parameter has {name}, which is not html, markdown or plaintext"
            ),
        }
    }
}

impl Error for ParseQueryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.kind {
            QueryErrorKind::Filter(error) => Some(error),
            _ => None,
        }
    }
}
