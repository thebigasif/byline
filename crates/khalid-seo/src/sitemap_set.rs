//! Sitemap files per section and language, split at the protocol's
//! limits, and the index that lists them.

use std::collections::BTreeMap;
use std::mem;

use khalid_core::{AbsoluteUrl, Locale};

use crate::sitemap::{URLSET_CLOSE, URLSET_OPEN, entry_for, is_listed, write_entry};
use crate::{PageKind, SeoError, SeoPage, SeoSite, Sitemap, SitemapEntry, SitemapIndex, UrlPath};

/// The protocol's limit on addresses in one sitemap file.
const MAX_URLS: usize = 50_000;

/// The protocol's limit on one sitemap file, uncompressed: 50 MiB.
const MAX_BYTES: usize = 52_428_800;

/// Which sitemap file a page goes in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum SitemapSection {
    /// Posts.
    Posts,
    /// The home page, standalone pages and glossary terms.
    Pages,
    /// Author, tag, category, collection and other archive pages.
    Archives,
}

impl SitemapSection {
    pub(crate) fn of(kind: PageKind) -> Self {
        match kind {
            PageKind::Post => Self::Posts,
            PageKind::Home | PageKind::Page | PageKind::AiPage | PageKind::GlossaryTerm => {
                Self::Pages
            }
            PageKind::Author
            | PageKind::Tag
            | PageKind::Category
            | PageKind::Collection
            | PageKind::Date
            | PageKind::Search
            | PageKind::GlossaryIndex
            | PageKind::NotFound => Self::Archives,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Posts => "posts",
            Self::Pages => "pages",
            Self::Archives => "archives",
        }
    }
}

/// One sitemap file and where it is served.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SitemapFile {
    /// Its path inside the mount folder, such as `/sitemaps/posts-en-1.xml`.
    pub path: UrlPath,
    /// Its public address.
    pub url: AbsoluteUrl,
    /// What it lists.
    pub sitemap: Sitemap,
}

/// Every sitemap file of a site, and the index that lists them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SitemapSet {
    files: Vec<SitemapFile>,
    index: SitemapIndex,
}

/// Cuts entries into files of at most 50,000 addresses and 50 MiB each.
fn split(entries: Vec<SitemapEntry>) -> Vec<Vec<SitemapEntry>> {
    let fixed = URLSET_OPEN.len() + URLSET_CLOSE.len();
    let mut chunks = Vec::new();
    let mut current = Vec::new();
    let mut bytes = fixed;
    let mut buffer = String::new();
    for entry in entries {
        buffer.clear();
        write_entry(&entry, &mut buffer);
        if !current.is_empty() && (current.len() == MAX_URLS || bytes + buffer.len() > MAX_BYTES) {
            chunks.push(mem::take(&mut current));
            bytes = fixed;
        }
        bytes += buffer.len();
        current.push(entry);
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

impl SitemapSet {
    /// One file per section and language, in the site's language order,
    /// numbered from 1 when a file must be split, and the index of them all.
    ///
    /// # Errors
    ///
    /// [`SeoError::Url`] when an address cannot be built.
    pub fn from_pages(site: &SeoSite, pages: &[SeoPage]) -> Result<Self, SeoError> {
        let rank = |language: &Locale| {
            site.languages
                .iter()
                .position(|declared| declared == language)
                .unwrap_or(site.languages.len())
        };
        let mut groups: BTreeMap<(SitemapSection, usize, Locale), Vec<SitemapEntry>> =
            BTreeMap::new();
        for page in pages.iter().filter(|page| is_listed(site, page)) {
            let key = (
                SitemapSection::of(page.kind),
                rank(&page.language),
                page.language.clone(),
            );
            groups.entry(key).or_default().push(entry_for(site, page)?);
        }
        let mut files = Vec::new();
        let mut index = SitemapIndex::default();
        for ((section, _, language), entries) in groups {
            for (number, chunk) in (1..).zip(split(entries)) {
                let name = format!(
                    "/sitemaps/{}-{}-{number}.xml",
                    section.as_str(),
                    language.as_str().to_ascii_lowercase()
                );
                let path = UrlPath::try_new(&name)?;
                let url = site.url_for(&site.default_language, &path)?;
                let lastmod = chunk.iter().filter_map(|entry| entry.lastmod).max();
                index.push(url.clone(), lastmod);
                files.push(SitemapFile {
                    path,
                    url,
                    sitemap: Sitemap::from_entries(chunk),
                });
            }
        }
        Ok(Self { files, index })
    }

    /// The files, in order.
    pub fn files(&self) -> &[SitemapFile] {
        &self.files
    }

    /// The index of the files.
    pub fn index(&self) -> &SitemapIndex {
        &self.index
    }
}
