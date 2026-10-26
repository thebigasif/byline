//! The HTML sitemap: a page for readers that links every listed page in
//! one language, grouped by section. Templates render it.

use khalid_core::{AbsoluteUrl, Locale};

use crate::sitemap::is_listed;
use crate::{SeoError, SeoPage, SeoSite, SitemapSection};

/// One link on the HTML sitemap.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct HtmlSitemapLink {
    /// The page's title, or the site name for the home page.
    pub title: String,
    /// The page's address.
    pub url: AbsoluteUrl,
}

/// The links of one section, sorted by title.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct HtmlSitemapGroup {
    /// Which section the links belong to.
    pub section: SitemapSection,
    /// The links, by title without regard to case.
    pub links: Vec<HtmlSitemapLink>,
}

/// The HTML sitemap of one language.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct HtmlSitemap {
    /// The language of the page and its links.
    pub language: Locale,
    /// The sections that have links, in a fixed order: posts, pages, archives.
    pub groups: Vec<HtmlSitemapGroup>,
}

impl HtmlSitemap {
    /// The HTML sitemap of `language`: every page the XML sitemap lists in
    /// that language, grouped by section and sorted by title.
    ///
    /// # Errors
    ///
    /// [`SeoError::Url`] when a page's address cannot be built.
    pub fn from_pages(
        site: &SeoSite,
        pages: &[SeoPage],
        language: &Locale,
    ) -> Result<Self, SeoError> {
        let mut groups: Vec<HtmlSitemapGroup> = Vec::new();
        for page in pages
            .iter()
            .filter(|page| &page.language == language && is_listed(site, page))
        {
            let title = if page.title.is_empty() {
                site.name.clone()
            } else {
                page.title.clone()
            };
            let link = HtmlSitemapLink {
                title,
                url: site.url_for(&page.language, &page.path)?,
            };
            let section = SitemapSection::of(page.kind);
            match groups.iter_mut().find(|group| group.section == section) {
                Some(group) => group.links.push(link),
                None => groups.push(HtmlSitemapGroup {
                    section,
                    links: vec![link],
                }),
            }
        }
        groups.sort_by_key(|group| group.section);
        for group in &mut groups {
            group.links.sort_by(|a, b| {
                (a.title.to_lowercase(), a.url.as_str())
                    .cmp(&(b.title.to_lowercase(), b.url.as_str()))
            });
        }
        Ok(Self {
            language: language.clone(),
            groups,
        })
    }
}
