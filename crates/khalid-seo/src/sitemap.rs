//! The XML sitemap: every published, indexable page, with its alternates.

use chrono::{DateTime, Utc};
use khalid_core::AbsoluteUrl;

use crate::escape::escape_markup;
use crate::social::rfc3339;
use crate::{
    Alternate, Indexing, PageKind, PublishState, SeoError, SeoPage, SeoSite, SitemapListing, Video,
};

/// Google reads at most this many images per page in a sitemap.
const MAX_IMAGES_PER_URL: usize = 1000;

/// The video sitemap schema's limit on a description, in characters.
const MAX_VIDEO_DESCRIPTION: usize = 2048;

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
    /// The images in the page, at most 1,000.
    pub images: Vec<AbsoluteUrl>,
    /// The videos in the page.
    pub videos: Vec<Video>,
}

fn element(out: &mut String, indent: &str, name: &str, text: &str) {
    out.push_str(indent);
    out.push('<');
    out.push_str(name);
    out.push('>');
    escape_markup(text, out);
    out.push_str("</");
    out.push_str(name);
    out.push_str(">\n");
}

fn write_video(video: &Video, out: &mut String) {
    let inner = "      ";
    out.push_str("    <video:video>\n");
    element(out, inner, "video:thumbnail_loc", video.thumbnail.as_str());
    element(out, inner, "video:title", &video.title);
    let description: String = video
        .description
        .chars()
        .take(MAX_VIDEO_DESCRIPTION)
        .collect();
    element(out, inner, "video:description", &description);
    if let Some(file) = video.file() {
        element(out, inner, "video:content_loc", file.as_str());
    }
    if let Some(player) = video.player() {
        element(out, inner, "video:player_loc", player.as_str());
    }
    if let Some(duration) = video.duration {
        let seconds = duration.as_secs().max(1).to_string();
        element(out, inner, "video:duration", &seconds);
    }
    if let Some(uploaded) = &video.uploaded {
        element(out, inner, "video:publication_date", &rfc3339(uploaded));
    }
    out.push_str("    </video:video>\n");
}

pub(crate) fn write_entry(entry: &SitemapEntry, out: &mut String) {
    out.push_str("  <url>\n");
    element(out, "    ", "loc", entry.loc.as_str());
    if let Some(lastmod) = &entry.lastmod {
        element(out, "    ", "lastmod", &rfc3339(lastmod));
    }
    for alternate in &entry.alternates {
        out.push_str("    <xhtml:link rel=\"alternate\" hreflang=\"");
        escape_markup(alternate.hreflang.as_str(), out);
        out.push_str("\" href=\"");
        escape_markup(alternate.url.as_str(), out);
        out.push_str("\"/>\n");
    }
    for image in &entry.images {
        out.push_str("    <image:image>\n");
        element(out, "      ", "image:loc", image.as_str());
        out.push_str("    </image:image>\n");
    }
    for video in &entry.videos {
        write_video(video, out);
    }
    out.push_str("  </url>\n");
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
            .map(|page| entry_for(site, page))
            .collect::<Result<Vec<_>, SeoError>>()?;
        Ok(Self { entries })
    }

    /// A sitemap of entries already chosen and ordered.
    pub(crate) fn from_entries(entries: Vec<SitemapEntry>) -> Self {
        Self { entries }
    }

    /// The entries, in order.
    pub fn entries(&self) -> &[SitemapEntry] {
        &self.entries
    }

    /// The sitemap as a UTF-8 XML document.
    pub fn to_xml(&self) -> String {
        let mut out = String::from(URLSET_OPEN);
        for entry in &self.entries {
            write_entry(entry, &mut out);
        }
        out.push_str(URLSET_CLOSE);
        out
    }
}

/// The start of a `urlset` document, with every namespace an entry may use.
pub(crate) const URLSET_OPEN: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
     <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\" \
     xmlns:xhtml=\"http://www.w3.org/1999/xhtml\" \
     xmlns:image=\"http://www.google.com/schemas/sitemap-image/1.1\" \
     xmlns:video=\"http://www.google.com/schemas/sitemap-video/1.1\">\n";

/// The end of a `urlset` document.
pub(crate) const URLSET_CLOSE: &str = "</urlset>\n";

/// The sitemap entry for a listed page.
pub(crate) fn entry_for(site: &SeoSite, page: &SeoPage) -> Result<SitemapEntry, SeoError> {
    Ok(SitemapEntry {
        loc: site.url_for(&page.language, &page.path)?,
        lastmod: page.modified.or(page.published),
        alternates: page.alternates.clone(),
        images: page
            .images
            .iter()
            .take(MAX_IMAGES_PER_URL)
            .cloned()
            .collect(),
        videos: page.videos.clone(),
    })
}
