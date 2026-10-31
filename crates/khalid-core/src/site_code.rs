//! `[code]` in `site.toml`: the site's own code for every page, such as a
//! tag manager or an analytics snippet, and whether it waits for each
//! reader's consent (KH-SEO-12).

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{HeadCode, HeadCodeError};

/// HTML for the end of every page's body: only `script` elements, and
/// comments, which are dropped.
///
/// It is checked and written again as [`HeadCode`] is, and each `script`
/// gets the page's nonce when it is drawn ([`BodyCode::to_html`]).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct BodyCode(HeadCode);

impl BodyCode {
    /// Reads `html` and keeps it in its checked form.
    ///
    /// # Errors
    ///
    /// Returns [`BodyCodeError::Element`] for an element other than
    /// `script`, and [`BodyCodeError::Code`] for the other faults
    /// [`HeadCode::try_new`] finds.
    pub fn try_new(html: &str) -> Result<Self, BodyCodeError> {
        HeadCode::scripts_only(html)
            .map(Self)
            .map_err(|error| match error {
                HeadCodeError::Element { name } => BodyCodeError::Element { name },
                other => BodyCodeError::Code(other),
            })
    }

    /// Whether there is nothing to write, for body code that held only
    /// comments or spaces.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The body code as HTML, with `nonce` on each `script`.
    pub fn to_html(&self, nonce: &str) -> String {
        self.0.to_html(nonce)
    }
}

impl fmt::Display for BodyCode {
    /// The checked form, without nonces.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Why body code was refused, naming what to change.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum BodyCodeError {
    /// An element other than `script`.
    #[error(
        "body code may hold only script, not {name}; put meta, link and style in the head code"
    )]
    Element {
        /// The element's name.
        name: String,
    },
    /// A fault head code has too, such as a script never closed.
    #[error("{}", body_words(.0))]
    Code(HeadCodeError),
}

/// The head code fault `error` as body code reads it.
fn body_words(error: &HeadCodeError) -> String {
    error.to_string().replacen("head code", "body code", 1)
}

impl TryFrom<String> for BodyCode {
    type Error = BodyCodeError;

    fn try_from(html: String) -> Result<Self, Self::Error> {
        Self::try_new(&html)
    }
}

impl From<BodyCode> for String {
    fn from(code: BodyCode) -> Self {
        code.to_string()
    }
}

/// Whether the site's code waits until a reader agrees to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Consent {
    /// Every page shows a banner, and the code runs only for a reader who
    /// pressed Allow. A reader who has not answered, or said no, gets
    /// none of it. A post's own head code then waits too.
    #[default]
    Ask,
    /// The code runs for every reader, with no banner, for code that sets
    /// no cookie and tracks no one.
    NotNeeded,
}

/// The site's own code from `[code]`: what goes at the end of every page's
/// head and body, and whether it waits for consent.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct SiteCode {
    /// Code for the end of every page's head.
    pub head: Option<HeadCode>,
    /// Code for the end of every page's body.
    pub body: Option<BodyCode>,
    /// Whether it waits for each reader's consent.
    #[serde(default)]
    pub consent: Consent,
}

impl SiteCode {
    /// Whether the site has no code of its own, or only comments.
    pub fn is_empty(&self) -> bool {
        self.head.as_ref().is_none_or(HeadCode::is_empty)
            && self.body.as_ref().is_none_or(BodyCode::is_empty)
    }

    /// Whether pages show the consent banner and hold the code back until a
    /// reader agrees: the site has code, and asks.
    pub fn waits_for_consent(&self) -> bool {
        self.consent == Consent::Ask && !self.is_empty()
    }
}
