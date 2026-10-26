//! The front matter of every content type other than posts.
//!
//! Fields the linter checks may be missing here, so the linter can explain
//! them.
//! Unknown fields and values of the wrong kind are refused.

use khalid_core::{AbsoluteUrl, Slug};
use serde::Deserialize;

/// A page, such as About, in `pages/`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct PageFront {
    /// The headline.
    pub title: Option<String>,
    /// The last part of the address.
    pub slug: Option<Slug>,
    /// The summary for search results.
    pub description: Option<String>,
    /// The key shared by the versions of this page in other languages.
    pub translation: Option<String>,
}

/// An author, in `authors/`. The body is the bio.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct AuthorFront {
    /// The name shown on posts.
    pub name: Option<String>,
    /// The slug posts use in their `author` field.
    pub slug: Option<Slug>,
    /// One line on what the author does.
    pub role: Option<String>,
    /// Profiles elsewhere, written into the author's schema as `sameAs`.
    #[serde(default)]
    pub same_as: Vec<AbsoluteUrl>,
}

/// A glossary term, in `glossary/`. The body is the definition.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct GlossaryFront {
    /// The term as it is written.
    pub term: Option<String>,
    /// The slug wikilinks use, such as `[[topical-authority]]`.
    pub slug: Option<Slug>,
    /// The key shared by the term in other languages.
    pub translation: Option<String>,
}
