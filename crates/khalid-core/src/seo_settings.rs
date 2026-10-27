//! The SEO settings of a site, from the `[seo]` table of `site.toml`.

use serde::Deserialize;

use crate::{AbsoluteUrl, IndexNowKey, Indexing};

/// The SEO settings. Every one has a default, so the table may be left out:
///
/// ```toml
/// [seo.index]
/// tags = "noindex"
/// thin_archives_below = 5
///
/// [seo.indexnow]
/// key = "3f9c0b5e2a8d4c71"
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[non_exhaustive]
pub struct SeoSettings {
    /// Which archive pages search engines may list, from `[seo.index]`.
    pub index: ArchiveIndex,
    /// Where changed pages are sent, from `[seo.indexnow]`. `None`, the
    /// default, sends nothing.
    pub indexnow: Option<IndexNowSettings>,
}

/// `IndexNow`, switched on by its table in `site.toml`.
///
/// `IndexNow` tells Bing, Yandex, Seznam, Naver and Yep about a changed page at
/// once, instead of waiting for their next crawl. The key is required; the
/// endpoint defaults to the shared one, which passes each request on to every
/// search engine that takes part.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct IndexNowSettings {
    /// The key the site serves in its key file.
    pub key: IndexNowKey,
    /// Where the changed addresses are sent. `None` for
    /// [`IndexNowSettings::SHARED_ENDPOINT`].
    #[serde(default)]
    pub endpoint: Option<AbsoluteUrl>,
}

impl IndexNowSettings {
    /// The shared `IndexNow` endpoint, used when `endpoint` is left out.
    pub const SHARED_ENDPOINT: &str = "https://api.indexnow.org/indexnow";

    /// `IndexNow` with `key`, sending to the shared endpoint.
    pub fn new(key: IndexNowKey) -> Self {
        Self {
            key,
            endpoint: None,
        }
    }
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
