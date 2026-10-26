//! The rest of the site, for rules that compare a post with its neighbours.

use khalid_core::{Locale, Slug, SlugMode};
use serde::Deserialize;

use crate::LintPost;

/// What a rule can read about the whole site.
///
/// [`LintSite`] is the plain implementation. Task 1.15 implements it for the
/// content track's site and rendered output, so the server lints without
/// copying every post.
pub trait SiteView {
    /// Every post on the site, in every language.
    fn posts(&self) -> &[LintPost];

    /// Every page that is not a post, such as "About".
    fn pages(&self) -> &[PageRef];

    /// Every term the glossary defines, in every language.
    fn glossary(&self) -> &[GlossaryTerm];

    /// The links on the home pages, such as the menu and the latest posts,
    /// as site paths. Empty when unknown, and the rules that need them skip.
    fn home_links(&self) -> &[String] {
        &[]
    }

    /// The languages the site declares. Empty when unknown, and the rules
    /// that need them skip.
    fn locales(&self) -> &[Locale] {
        &[]
    }

    /// How the site writes slugs, from `slugs` in `site.toml`. `None` when
    /// unknown, and the rules that need it skip.
    fn slug_mode(&self) -> Option<SlugMode> {
        None
    }

    /// The site's own host name, such as `example.com`, from its public
    /// address. `None` when unknown, and the rules that need it skip.
    fn host(&self) -> Option<&str> {
        None
    }

    /// The host names of the other sites this Khalid process serves, such as
    /// `shop.example.org`. Empty for a single site.
    fn sibling_hosts(&self) -> &[String] {
        &[]
    }

    /// The site's collections from `routes.toml`, in the file's order.
    /// Empty when unknown, and the rules that need them skip.
    fn collections(&self) -> &[LintCollection] {
        &[]
    }
}

/// A site's posts, pages and glossary, held in memory.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[non_exhaustive]
pub struct LintSite {
    /// Every post, in every language.
    pub posts: Vec<LintPost>,
    /// Every page that is not a post.
    pub pages: Vec<PageRef>,
    /// Every glossary term.
    pub glossary: Vec<GlossaryTerm>,
    /// The links on the home pages, as site paths.
    pub home_links: Vec<String>,
    /// The languages the site declares.
    pub locales: Vec<Locale>,
    /// How the site writes slugs, when known.
    pub slugs: Option<SlugMode>,
    /// The site's own host name, such as `example.com`, when known.
    pub host: Option<String>,
    /// The host names of the other sites in the same process.
    pub sibling_hosts: Vec<String>,
    /// The site's collections.
    pub collections: Vec<LintCollection>,
}

impl SiteView for LintSite {
    fn posts(&self) -> &[LintPost] {
        &self.posts
    }

    fn pages(&self) -> &[PageRef] {
        &self.pages
    }

    fn glossary(&self) -> &[GlossaryTerm] {
        &self.glossary
    }

    fn home_links(&self) -> &[String] {
        &self.home_links
    }

    fn locales(&self) -> &[Locale] {
        &self.locales
    }

    fn slug_mode(&self) -> Option<SlugMode> {
        self.slugs
    }

    fn host(&self) -> Option<&str> {
        self.host.as_deref()
    }

    fn sibling_hosts(&self) -> &[String] {
        &self.sibling_hosts
    }

    fn collections(&self) -> &[LintCollection] {
        &self.collections
    }
}

/// A collection: a folder of posts and the address pattern they share.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct LintCollection {
    /// The collection's name, which is also its folder, such as `guides`.
    pub name: String,
    /// The address pattern of its posts, such as `/news/{year}/{slug}/`,
    /// with the placeholders `routes.toml` allows.
    pub permalink: String,
}

impl LintCollection {
    /// The collection `name` with the address pattern `permalink`.
    pub fn new(name: impl Into<String>, permalink: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            permalink: permalink.into(),
        }
    }
}

/// A page on the site that is not a post.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[non_exhaustive]
pub struct PageRef {
    /// The page's language.
    pub locale: Locale,
    /// The last part of the page's address.
    pub slug: Slug,
}

impl PageRef {
    /// The page `slug` in `locale`.
    pub fn new(locale: Locale, slug: Slug) -> Self {
        Self { locale, slug }
    }
}

/// A term the site's glossary defines.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[non_exhaustive]
pub struct GlossaryTerm {
    /// The term's language.
    pub locale: Locale,
    /// The term as readers write it, such as "internal link".
    pub term: String,
    /// The last part of the term's address.
    pub slug: Slug,
}

impl GlossaryTerm {
    /// The term `term` in `locale`, defined at `slug`.
    pub fn new(locale: Locale, term: impl Into<String>, slug: Slug) -> Self {
        Self {
            locale,
            term: term.into(),
            slug,
        }
    }
}
