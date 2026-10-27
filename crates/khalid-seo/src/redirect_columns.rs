//! A redirect rule as four plain columns, the way a CSV row and a database
//! row both hold it.

use khalid_core::AbsoluteUrl;

use crate::{
    Destination, Move, PatternRedirect, PatternRule, Redirect, RedirectEntry, RedirectPattern,
    RedirectRule, SeoError, UrlPath,
};

/// How a rule's `from` matches addresses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Matching {
    /// `from` is one path.
    Exact,
    /// `from` is a regular expression over the whole path.
    Regex,
}

impl Matching {
    /// The name written in a list or a row: `exact` or `regex`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Regex => "regex",
        }
    }

    /// The matching named `name`, ignoring case and spaces around it.
    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim().to_lowercase().as_str() {
            "exact" => Some(Self::Exact),
            "regex" => Some(Self::Regex),
            _ => None,
        }
    }
}

/// A rule as plain columns: `from`, `to` (none for 410 and 451), the
/// status code, and how `from` matches.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RedirectColumns {
    /// The path, or the pattern as written.
    pub from: String,
    /// Where a move leads: a path, a web address, or a pattern's template.
    pub to: Option<String>,
    /// 301, 302, 307, 410 or 451.
    pub status: u16,
    /// How `from` matches.
    pub matching: Matching,
}

impl RedirectColumns {
    /// The columns of one rule.
    pub fn new(from: &str, to: Option<&str>, status: u16, matching: Matching) -> Self {
        Self {
            from: from.to_owned(),
            to: to.map(ToOwned::to_owned),
            status,
            matching,
        }
    }
}

impl RedirectEntry {
    /// The rule as plain columns.
    pub fn columns(&self) -> RedirectColumns {
        match self {
            Self::Exact(rule) => {
                let to = rule.redirect.location().map(ToString::to_string);
                let (from, status) = (rule.from.as_str(), rule.redirect.status());
                RedirectColumns::new(from, to.as_deref(), status, Matching::Exact)
            }
            Self::Pattern(rule) => {
                let (to, status) = match &rule.answer {
                    PatternRedirect::Move { kind, template } => (Some(template), kind.status()),
                    PatternRedirect::Gone => (None, 410),
                    PatternRedirect::LegalBlock => (None, 451),
                };
                let from = rule.pattern.as_str();
                RedirectColumns::new(from, to.map(String::as_str), status, Matching::Regex)
            }
        }
    }

    /// The rule the columns describe, checked.
    ///
    /// # Errors
    ///
    /// [`SeoError::RedirectColumns`] for a status other than 301, 302, 307,
    /// 410 or 451, a move with no `to`, or a 410 or 451 with one. The error
    /// of [`UrlPath::try_new`], [`RedirectPattern::try_new`] or
    /// [`PatternRedirect::move_to`] for a refused path or pattern, and
    /// [`SeoError::RedirectAddress`] for a `to` that is neither a path nor a
    /// web address.
    pub fn from_columns(columns: &RedirectColumns) -> Result<Self, SeoError> {
        let problem = |problem| SeoError::RedirectColumns {
            from: columns.from.clone(),
            problem,
        };
        let kind = match columns.status {
            301 => Some(Move::Permanent),
            302 => Some(Move::Found),
            307 => Some(Move::Temporary),
            410 | 451 => None,
            _ => return Err(problem("has a status other than 301, 302, 307, 410 or 451")),
        };
        let to = columns
            .to
            .as_deref()
            .map(str::trim)
            .filter(|to| !to.is_empty());
        let (from, gone) = (columns.from.trim(), columns.status == 410);
        let moved = match (kind, to) {
            (Some(kind), Some(to)) => Some((kind, to)),
            (Some(_), None) => {
                return Err(problem(
                    "moves to nowhere: a 301, 302 or 307 needs a to address",
                ));
            }
            (None, Some(_)) => {
                return Err(problem("has a to address, which a 410 or 451 cannot use"));
            }
            (None, None) => None,
        };
        if columns.matching == Matching::Regex {
            let answer = match moved {
                Some((kind, to)) => PatternRedirect::move_to(kind, to)?,
                None if gone => PatternRedirect::Gone,
                None => PatternRedirect::LegalBlock,
            };
            let pattern = RedirectPattern::try_new(from)?;
            return Ok(Self::Pattern(PatternRule::new(pattern, answer)));
        }
        let redirect = match moved {
            Some((kind, to)) => Redirect::Move {
                kind,
                to: destination(to)?,
            },
            None if gone => Redirect::Gone,
            None => Redirect::LegalBlock,
        };
        Ok(Self::Exact(RedirectRule::new(
            UrlPath::try_new(from)?,
            redirect,
        )))
    }
}

/// A path on this site when `to` starts with `/`, otherwise a web address.
fn destination(to: &str) -> Result<Destination, SeoError> {
    if to.starts_with('/') {
        return Ok(Destination::Path(UrlPath::try_new(to)?));
    }
    AbsoluteUrl::parse(to)
        .map(Destination::External)
        .map_err(|source| SeoError::RedirectAddress {
            address: to.to_owned(),
            source,
        })
}
