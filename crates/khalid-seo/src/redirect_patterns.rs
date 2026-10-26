//! Regex redirect rules, such as every `/2019/05/<slug>/` to `/<slug>/`.

use khalid_core::AbsoluteUrl;
use regex::{Regex, RegexBuilder};

use crate::{Destination, Move, Redirect, SeoError, UrlPath};

/// The most memory one compiled pattern may take, so a pasted pattern
/// cannot use up the server's memory.
const PATTERN_SIZE_LIMIT: usize = 1 << 20;

/// A regular expression that must match the whole path.
///
/// Matching takes time in proportion to the path's length whatever the
/// pattern, so no pattern can stall the server.
#[derive(Debug, Clone)]
pub struct RedirectPattern {
    regex: Regex,
    source: String,
}

impl PartialEq for RedirectPattern {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source
    }
}

impl Eq for RedirectPattern {}

impl RedirectPattern {
    /// Compiles `text`, anchored at both ends.
    ///
    /// # Errors
    ///
    /// [`SeoError::RedirectPattern`] when `text` is not a valid pattern or
    /// compiles too large.
    pub fn try_new(text: &str) -> Result<Self, SeoError> {
        let regex = RegexBuilder::new(&format!("^(?:{text})$"))
            .size_limit(PATTERN_SIZE_LIMIT)
            .build()
            .map_err(|source| SeoError::RedirectPattern {
                pattern: text.to_owned(),
                source,
            })?;
        Ok(Self {
            regex,
            source: text.to_owned(),
        })
    }

    /// The pattern as written, without the anchors.
    pub fn as_str(&self) -> &str {
        &self.source
    }
}

/// What a path matching a pattern answers.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PatternRedirect {
    /// A move to an address built from the match: `$1` or `${1}` is the
    /// first group, `${name}` a named one.
    Move {
        /// Which kind of move.
        kind: Move,
        /// The new address, with group references.
        template: String,
    },
    /// 410.
    Gone,
    /// 451.
    LegalBlock,
}

impl PatternRedirect {
    /// A move to `template`, which must be a path or an `http(s)` address.
    ///
    /// # Errors
    ///
    /// [`SeoError::RedirectTemplate`] when `template` starts with neither
    /// `/` nor `http://` or `https://`.
    pub fn move_to(kind: Move, template: &str) -> Result<Self, SeoError> {
        let valid = template.starts_with('/')
            || template.starts_with("https://")
            || template.starts_with("http://");
        if valid {
            Ok(Self::Move {
                kind,
                template: template.to_owned(),
            })
        } else {
            Err(SeoError::RedirectTemplate {
                template: template.to_owned(),
            })
        }
    }
}

/// A pattern and what its matches answer.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PatternRule {
    /// The pattern.
    pub pattern: RedirectPattern,
    /// What a matching path answers.
    pub answer: PatternRedirect,
}

impl PatternRule {
    /// A rule for paths matching `pattern`.
    pub fn new(pattern: RedirectPattern, answer: PatternRedirect) -> Self {
        Self { pattern, answer }
    }

    /// The answer for `path`, or `None` when the pattern does not match or
    /// the address built from the match is not valid.
    pub(crate) fn apply(&self, path: &UrlPath) -> Option<Redirect> {
        let captures = self.pattern.regex.captures(path.as_str())?;
        match &self.answer {
            PatternRedirect::Gone => Some(Redirect::Gone),
            PatternRedirect::LegalBlock => Some(Redirect::LegalBlock),
            PatternRedirect::Move { kind, template } => {
                let mut built = String::new();
                captures.expand(template, &mut built);
                let to = if built.starts_with('/') {
                    Destination::Path(UrlPath::try_new(&built).ok()?)
                } else {
                    Destination::External(AbsoluteUrl::parse(&built).ok()?)
                };
                Some(Redirect::Move { kind: *kind, to })
            }
        }
    }
}
