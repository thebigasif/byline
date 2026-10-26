//! The rest of the site, for rules that compare a post with its neighbours.

use khalid_core::{Locale, Slug};

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
}

/// A site's posts, pages and glossary, held in memory.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct LintSite {
    /// Every post, in every language.
    pub posts: Vec<LintPost>,
    /// Every page that is not a post.
    pub pages: Vec<PageRef>,
    /// Every glossary term.
    pub glossary: Vec<GlossaryTerm>,
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
}

/// A page on the site that is not a post.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
