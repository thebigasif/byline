//! [`SeoPage`]: what the SEO outputs need to know about one page.

use khalid_core::Locale;

use crate::{MetaTemplate, UrlPath};

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
        }
    }
}
