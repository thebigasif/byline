//! The SEO settings of a site, from the `[seo]` table of `site.toml`.

use serde::Deserialize;

use crate::Indexing;

/// The SEO settings. Every one has a default, so the table may be left out:
///
/// ```toml
/// [seo.index]
/// tags = "noindex"
/// thin_archives_below = 5
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[non_exhaustive]
pub struct SeoSettings {
    /// Which archive pages search engines may list, from `[seo.index]`.
    pub index: ArchiveIndex,
}

/// Whether search engines may list each kind of archive page: `"index"` or
/// `"noindex"`.
///
/// By default tag, category and author archives are indexed, and date
/// archives and search results are not. A tag or author archive with fewer
/// posts than `thin_archives_below` is never indexed, because a page with
/// one or two links looks thin to a search engine.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[non_exhaustive]
pub struct ArchiveIndex {
    /// Tag archives, such as `/tag/crawl-budget/`.
    pub tags: Indexing,
    /// Category archives, such as `/category/technical-seo/`.
    pub categories: Indexing,
    /// Author archives, such as `/authors/asif/`.
    pub authors: Indexing,
    /// Date archives, such as `/2026/10/`.
    pub dates: Indexing,
    /// Search results pages.
    pub search: Indexing,
    /// A tag or author archive listing fewer posts than this is `noindex`.
    /// `0` turns the rule off.
    pub thin_archives_below: u32,
}

impl Default for ArchiveIndex {
    fn default() -> Self {
        Self {
            tags: Indexing::Index,
            categories: Indexing::Index,
            authors: Indexing::Index,
            dates: Indexing::NoIndex,
            search: Indexing::NoIndex,
            thin_archives_below: 3,
        }
    }
}
