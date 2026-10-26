//! The sitemap index: the one address that lists every sitemap file.

use chrono::{DateTime, Utc};
use khalid_core::AbsoluteUrl;

use crate::escape::escape_markup;
use crate::social::rfc3339;
use crate::{SeoError, SeoSite, UrlPath};

/// Where the index lives, inside the mount folder.
const INDEX_PATH: &str = "/sitemap.xml";

/// The address of the sitemap index: `sitemap.xml` in the mount folder.
///
/// # Errors
///
/// [`SeoError::Url`] when the address cannot be built.
pub fn sitemap_index_url(site: &SeoSite) -> Result<AbsoluteUrl, SeoError> {
    site.url_for(&site.default_language, &UrlPath::try_new(INDEX_PATH)?)
}

/// One `<sitemap>` in the index.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct IndexEntry {
    /// The sitemap file's address.
    pub loc: AbsoluteUrl,
    /// The latest change among the pages it lists.
    pub lastmod: Option<DateTime<Utc>>,
}

/// The sitemap index: the one address search engines and `robots.txt` need.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SitemapIndex {
    entries: Vec<IndexEntry>,
}

impl SitemapIndex {
    /// Adds a sitemap file at the end.
    pub fn push(&mut self, loc: AbsoluteUrl, lastmod: Option<DateTime<Utc>>) {
        self.entries.push(IndexEntry { loc, lastmod });
    }

    /// The listed files, in order.
    pub fn entries(&self) -> &[IndexEntry] {
        &self.entries
    }

    /// The index as a UTF-8 XML document.
    pub fn to_xml(&self) -> String {
        let mut out = String::from(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <sitemapindex xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
        );
        for entry in &self.entries {
            out.push_str("  <sitemap>\n    <loc>");
            escape_markup(entry.loc.as_str(), &mut out);
            out.push_str("</loc>\n");
            if let Some(lastmod) = &entry.lastmod {
                out.push_str("    <lastmod>");
                out.push_str(&rfc3339(lastmod));
                out.push_str("</lastmod>\n");
            }
            out.push_str("  </sitemap>\n");
        }
        out.push_str("</sitemapindex>\n");
        out
    }
}
