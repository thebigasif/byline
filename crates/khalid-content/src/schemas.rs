//! The front matter of every content type other than posts.
//!
//! Fields the linter checks, such as a case study's client or a changelog
//! entry's version, may be missing here so the linter can explain them.
//! Unknown fields and values of the wrong kind are refused.

use chrono::NaiveDate;
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
    /// The author's photo for bylines, cards and the author page: a file
    /// beside the author's file, such as `asif.jpg`, a media path or a
    /// full address.
    #[serde(default, deserialize_with = "crate::post_fields::photo")]
    pub photo: Option<String>,
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

/// A case study, in `case-studies/`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct CaseStudyFront {
    /// The headline.
    pub title: Option<String>,
    /// The last part of the address.
    pub slug: Option<Slug>,
    /// The day the study is dated.
    #[serde(default, deserialize_with = "crate::post::date")]
    pub date: Option<NaiveDate>,
    /// The client the study is about.
    pub client: Option<String>,
    /// The result, in one line.
    pub outcome: Option<String>,
    /// The summary for search results.
    pub description: Option<String>,
    /// The key shared by the versions in other languages.
    pub translation: Option<String>,
}

/// A changelog entry, in `changelog/`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct ChangelogFront {
    /// The headline.
    pub title: Option<String>,
    /// The day the change shipped.
    #[serde(default, deserialize_with = "crate::post::date")]
    pub date: Option<NaiveDate>,
    /// The version it shipped in, such as `2.4.0`.
    pub version: Option<String>,
    /// The key shared by the versions in other languages.
    pub translation: Option<String>,
}

/// A snippet, in `snippets/`: a reusable group of blocks.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct SnippetFront {
    /// The name shown in the editor.
    pub title: Option<String>,
    /// The slug that places the snippet in a post.
    pub slug: Option<Slug>,
}

/// A collection's own front matter, in its `_index.md`. The body introduces
/// the collection on its list page.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct IndexFront {
    /// The list page's headline.
    pub title: Option<String>,
    /// The summary for search results.
    pub description: Option<String>,
}
