//! [`SeoPage`]: what the SEO outputs need to know about one page.

use chrono::{DateTime, Utc};
use khalid_core::{AbsoluteUrl, Locale};

use crate::{
    Alternate, MetaTemplate, RobotsDirectives, ShareImage, TranslationKey, UrlPath, Video,
};

/// The kind of page, which picks its title template and index setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum PageKind {
    /// The blog's front page.
    Home,
    /// One post.
    Post,
    /// A standalone page such as "About".
    Page,
    /// An author's archive.
    Author,
    /// A tag archive.
    Tag,
    /// A category archive.
    Category,
    /// A collection index such as `/guides/`.
    Collection,
    /// A date archive.
    Date,
    /// Search results.
    Search,
    /// The glossary's A to Z index.
    GlossaryIndex,
    /// One glossary term.
    GlossaryTerm,
    /// The page shown for an address that does not exist.
    NotFound,
}

/// Whether readers can see the page yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum PublishState {
    /// Live on the site.
    #[default]
    Published,
    /// Not finished, seen only in the admin.
    Draft,
    /// Finished, waiting for its publish time.
    Scheduled,
}

/// Whether the page appears in the XML sitemap when it is indexable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum SitemapListing {
    /// Listed, the default.
    #[default]
    Listed,
    /// Left out, such as a thank-you page after a form.
    Excluded,
}

/// What the SEO outputs need to know about one page.
///
/// Build it with [`SeoPage::new`], then fill the fields the page has.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SeoPage {
    /// The kind of page.
    pub kind: PageKind,
    /// The language the page is written in.
    pub language: Locale,
    /// The path inside the mount folder and language prefix, such as `/why-links-matter/`.
    pub path: UrlPath,
    /// The page's own title, the `{title}` variable.
    pub title: String,
    /// The summary, the `{excerpt}` variable.
    pub excerpt: String,
    /// The category name, the `{category}` variable.
    pub category: Option<String>,
    /// The author's name, the `{author}` variable.
    pub author: Option<String>,
    /// A title template for this page alone, in place of its kind's template.
    pub seo_title: Option<MetaTemplate>,
    /// A description template for this page alone.
    pub seo_description: Option<MetaTemplate>,
    /// The canonical address when it is not this page, such as the original of a syndicated post.
    pub canonical: Option<AbsoluteUrl>,
    /// Robots directives for this page alone, in place of the site's index policy.
    pub robots: Option<RobotsDirectives>,
    /// For an archive, how many posts it lists. A thin archive is not indexed.
    pub item_count: Option<u32>,
    /// The image link previews show, in place of the site's default.
    pub share_image: Option<ShareImage>,
    /// When the page was first published.
    pub published: Option<DateTime<Utc>>,
    /// When the page last changed in a way a reader would notice.
    pub modified: Option<DateTime<Utc>>,
    /// The group this page's language versions share.
    pub translation: Option<TranslationKey>,
    /// Every language version of the page, itself included, then
    /// `x-default`. [`link_translations`](crate::link_translations) fills it.
    pub alternates: Vec<Alternate>,
    /// Whether readers can see the page yet.
    pub state: PublishState,
    /// Whether the page appears in the XML sitemap.
    pub sitemap: SitemapListing,
    /// The images in the page's content, for the image sitemap.
    pub images: Vec<AbsoluteUrl>,
    /// The videos in the page's content, for the video sitemap.
    pub videos: Vec<Video>,
}

impl SeoPage {
    /// A page with a kind, a language, a path and a title, and nothing else set.
    pub fn new(kind: PageKind, language: Locale, path: UrlPath, title: &str) -> Self {
        Self {
            kind,
            language,
            path,
            title: title.to_owned(),
            excerpt: String::new(),
            category: None,
            author: None,
            seo_title: None,
            seo_description: None,
            canonical: None,
            robots: None,
            item_count: None,
            share_image: None,
            published: None,
            modified: None,
            translation: None,
            alternates: Vec::new(),
            state: PublishState::Published,
            sitemap: SitemapListing::Listed,
            images: Vec::new(),
            videos: Vec::new(),
        }
    }
}
