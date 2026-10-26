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
/// The most names in `include` or `fields`.
const MAX_NAMES: usize = 50;
/// The most keys in `order`.
const MAX_ORDER_KEYS: usize = 5;

/// The list parameters as they arrive in the query string, before any
/// checking. Other parameters, such as `key`, are left to the caller.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[non_exhaustive]
pub struct RawListQuery {
    /// `filter=tag:news+featured:true`
    pub filter: Option<String>,
    /// `include=authors,tags`
    pub include: Option<String>,
    /// `fields=title,slug`
    pub fields: Option<String>,
    /// `order=published_at desc,title`
    pub order: Option<String>,
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
    include: Vec<String>,
    fields: Vec<String>,
    order: Vec<OrderKey>,
    formats: Vec<Format>,
    limit: Limit,
    page: u32,
}

impl ListQuery {
    /// The filter, if one was given.
    pub fn filter(&self) -> Option<&Filter> {
        self.filter.as_ref()
    }
    /// Related resources to embed, such as `authors`, without repeats.
    pub fn include(&self) -> &[String] {
        &self.include
    }
    /// The only fields to return, without repeats; empty means all.
    pub fn fields(&self) -> &[String] {
        &self.fields
    }
    /// Sort keys, most important first; empty means the resource's own order.
    pub fn order(&self) -> &[OrderKey] {
        &self.order
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
            include: raw
                .include
                .as_deref()
                .map_or(Ok(Vec::new()), |text| names("include", text))?,
            fields: raw
                .fields
                .as_deref()
                .map_or(Ok(Vec::new()), |text| names("fields", text))?,
            order: raw.order.as_deref().map_or(Ok(Vec::new()), order)?,
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

/// Which way a sort key runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Smallest first, the default.
    Ascending,
    /// Largest first.
    Descending,
}

/// One sort key, such as `published_at desc`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderKey {
    field: String,
    direction: Direction,
}

impl OrderKey {
    /// The field sorted by.
    pub fn field(&self) -> &str {
        &self.field
    }
    /// Which way.
    pub fn direction(&self) -> Direction {
        self.direction
    }
}

/// A comma list of field names, without repeats.
fn names(parameter: &'static str, text: &str) -> Result<Vec<String>, ParseQueryError> {
    let fail = |kind| ParseQueryError::new(parameter, kind);
    let given: Vec<&str> = text.split(',').map(str::trim).collect();
    if given.len() > MAX_NAMES {
        return Err(fail(QueryErrorKind::TooMany(MAX_NAMES)));
    }
    let mut found: Vec<String> = Vec::new();
    for name in given {
        check_name(name).map_err(fail)?;
        if !found.iter().any(|seen| seen == name) {
            found.push(name.to_owned());
        }
    }
    Ok(found)
}

/// A field name: a lowercase letter or `_`, then letters, digits, `_` or `.`.
fn check_name(name: &str) -> Result<(), QueryErrorKind> {
    let mut chars = name.chars();
    let first = chars.next().ok_or(QueryErrorKind::EmptyName)?;
    let rest = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '.');
    if (first.is_ascii_lowercase() || first == '_') && chars.all(rest) {
        Ok(())
    } else {
        Err(QueryErrorKind::BadName(name.to_owned()))
    }
}

/// `field [asc|desc]`, comma separated.
fn order(text: &str) -> Result<Vec<OrderKey>, ParseQueryError> {
    let fail = |kind| ParseQueryError::new("order", kind);
    let mut keys = Vec::new();
    for item in text.split(',').map(str::trim) {
        let words: Vec<&str> = item.split_whitespace().collect();
        let (field, direction) = match words.as_slice() {
            [field] => (*field, Direction::Ascending),
            [field, word] if word.eq_ignore_ascii_case("asc") => (*field, Direction::Ascending),
            [field, word] if word.eq_ignore_ascii_case("desc") => (*field, Direction::Descending),
            [_, word] => return Err(fail(QueryErrorKind::BadDirection((*word).to_owned()))),
            [] => return Err(fail(QueryErrorKind::EmptyName)),
            _ => return Err(fail(QueryErrorKind::BadOrderKey(item.to_owned()))),
        };
        check_name(field).map_err(fail)?;
        keys.push(OrderKey {
            field: field.to_owned(),
            direction,
        });
    }
    if keys.len() > MAX_ORDER_KEYS {
        return Err(fail(QueryErrorKind::TooMany(MAX_ORDER_KEYS)));
    }
    Ok(keys)
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
    /// A name that is not a field name.
    BadName(String),
    /// An empty name between commas.
    EmptyName,
    /// A direction other than asc or desc.
    BadDirection(String),
    /// An order key with more than a field and a direction.
    BadOrderKey(String),
    /// More names than allowed.
    TooMany(usize),
}

impl fmt::Display for ParseQueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parameter = self.parameter;
        match &self.kind {
            QueryErrorKind::BadName(name) => write!(
                f,
                "the {parameter} parameter has {name}, which is not a field name"
            ),
            QueryErrorKind::EmptyName => write!(f, "the {parameter} parameter has an empty name"),
            QueryErrorKind::BadDirection(word) => write!(
                f,
                "the order parameter has {word}, which is not asc or desc"
            ),
            QueryErrorKind::BadOrderKey(item) => write!(
                f,
                "the order parameter has {item}, which is not a field and a direction"
            ),
            QueryErrorKind::TooMany(max) => {
                write!(f, "the {parameter} parameter has more than {max} names")
            }
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
