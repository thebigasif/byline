//! The XML sitemap: every published, indexable page, with its alternates.

use chrono::{DateTime, SecondsFormat, Utc};
use khalid_core::AbsoluteUrl;

use crate::escape::escape_markup;
use crate::{
    Alternate, Indexing, PageKind, PublishState, SeoError, SeoPage, SeoSite, SitemapListing,
};

/// One `<url>` in a sitemap.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SitemapEntry {
    /// The page's address.
    pub loc: AbsoluteUrl,
    /// When the page last changed, or was published if it never changed.
    pub lastmod: Option<DateTime<Utc>>,
    /// The page's language versions, as `xhtml:link` alternates.
    pub alternates: Vec<Alternate>,
}

/// A `urlset` sitemap file, in the order its pages were given.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Sitemap {
    entries: Vec<SitemapEntry>,
}

/// Whether `page` belongs in a sitemap: published, listed, indexable, its
/// own canonical, and not a search or not-found page.
pub(crate) fn is_listed(site: &SeoSite, page: &SeoPage) -> bool {
    page.state == PublishState::Published
        && page.sitemap == SitemapListing::Listed
        && page.canonical.is_none()
        && !matches!(page.kind, PageKind::Search | PageKind::NotFound)
        && site.indexing.directives_for(page).indexing == Indexing::Index
}

impl Sitemap {
    /// The sitemap of every page in `pages` that belongs in one.
    ///
    /// A page is listed when it is published, not excluded, indexable under
    /// the site's index policy, and not pointing to another canonical. Call
    /// [`link_translations`](crate::link_translations) first to list the
    /// language alternates.
    ///
    /// # Errors
    ///
    /// [`SeoError::Url`] when a page's address cannot be built.
    pub fn from_pages(site: &SeoSite, pages: &[SeoPage]) -> Result<Self, SeoError> {
        let entries = pages
            .iter()
            .filter(|page| is_listed(site, page))
            .map(|page| {
                Ok(SitemapEntry {
                    loc: site.url_for(&page.language, &page.path)?,
                    lastmod: page.modified.or(page.published),
                    alternates: page.alternates.clone(),
                })
            })
            .collect::<Result<Vec<_>, SeoError>>()?;
        Ok(Self { entries })
    }

    /// The entries, in order.
    pub fn entries(&self) -> &[SitemapEntry] {
        &self.entries
    }

    /// The sitemap as a UTF-8 XML document.
    pub fn to_xml(&self) -> String {
        let mut out = String::from(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\" \
             xmlns:xhtml=\"http://www.w3.org/1999/xhtml\">\n",
        );
        for entry in &self.entries {
            out.push_str("  <url>\n    <loc>");
            escape_markup(entry.loc.as_str(), &mut out);
            out.push_str("</loc>\n");
            if let Some(lastmod) = &entry.lastmod {
                out.push_str("    <lastmod>");
                out.push_str(&lastmod.to_rfc3339_opts(SecondsFormat::Secs, true));
                out.push_str("</lastmod>\n");
            }
            for alternate in &entry.alternates {
                out.push_str("    <xhtml:link rel=\"alternate\" hreflang=\"");
                escape_markup(alternate.hreflang.as_str(), &mut out);
                out.push_str("\" href=\"");
                escape_markup(alternate.url.as_str(), &mut out);
                out.push_str("\"/>\n");
            }
            out.push_str("  </url>\n");
        }
        out.push_str("</urlset>\n");
        out
    }
}
