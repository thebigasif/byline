//! What a `WordPress` export holds, as read from the file and before any of it
//! becomes Khalid content.
//!
//! Text is kept as `WordPress` wrote it: slugs may be percent-encoded and
//! names may hold HTML entities. Turning them into Khalid's types is a later
//! step, so this model loses nothing the file says about people and terms.
//! Authors' email addresses are personal data the blog never shows, so they
//! are not read (RULES.md BL-RUN-5).

use khalid_core::AbsoluteUrl;

/// Everything read from one export file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Export {
    /// The site the export came from.
    pub site: Site,
    /// The people who wrote, in file order.
    pub authors: Vec<Author>,
    /// The categories, in file order. A parent comes before its children.
    pub categories: Vec<Term>,
    /// The tags, in file order.
    pub tags: Vec<Term>,
}

/// The site the export came from.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Site {
    /// The site's name.
    pub title: String,
    /// The site's home page, which every old address starts with.
    pub address: AbsoluteUrl,
    /// The language tag `WordPress` was set to, such as `en-US`.
    pub language: Option<String>,
}

/// A person who wrote on the site.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Author {
    /// The sign-in name that items name as their creator.
    pub login: String,
    /// The name shown on posts.
    pub display_name: String,
}

/// A category or a tag.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Term {
    /// The id, which SEO plugins use to name a post's primary category.
    /// Older exports leave it out.
    pub id: Option<u64>,
    /// The slug as `WordPress` wrote it, which may be percent-encoded.
    pub slug: String,
    /// The name shown to readers.
    pub name: String,
    /// The parent category's slug, for a category that has one.
    pub parent: Option<String>,
}
