//! One post as the linter sees it.

use chrono::NaiveDate;
use khalid_core::{Locale, Slug};

/// A post, reduced to what the rules read.
///
/// The content track builds it from a parsed and rendered post; `SerpDad` builds
/// it from a draft. Fields that can be missing in a draft are `Option`, so a
/// rule sees the gap instead of a made-up value. Start from [`LintPost::new`]
/// and set the fields you have.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LintPost {
    /// The language the post is written in.
    pub locale: Locale,
    /// The last part of the post's address.
    pub slug: Slug,
    /// The title from the header, which becomes the page's main headline.
    pub title: Option<String>,
    /// The publish date from the header.
    pub date: Option<NaiveDate>,
    /// The author from the header.
    pub author: Option<String>,
    /// The headings in the body, in order. The title is not one of them.
    pub headings: Vec<Heading>,
    /// The links in the body, in order.
    pub links: Vec<LintLink>,
    /// The images in the body, in order.
    pub images: Vec<Image>,
    /// How many words the body holds.
    pub word_count: usize,
    /// The body as plain text, with markup removed.
    pub body_text: String,
}

impl LintPost {
    /// An empty post with only its language and address set.
    pub fn new(locale: Locale, slug: Slug) -> Self {
        Self {
            locale,
            slug,
            title: None,
            date: None,
            author: None,
            headings: Vec::new(),
            links: Vec::new(),
            images: Vec::new(),
            word_count: 0,
            body_text: String::new(),
        }
    }
}

/// A heading in a post's body.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Heading {
    /// How deep the heading sits, from the main headline down.
    pub level: HeadingLevel,
    /// The heading's text.
    pub text: String,
}

impl Heading {
    /// A heading at `level` with `text`.
    pub fn new(level: HeadingLevel, text: impl Into<String>) -> Self {
        Self {
            level,
            text: text.into(),
        }
    }
}

/// The six heading levels of HTML, from `h1` to `h6`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HeadingLevel {
    /// The main headline, `#` in markdown.
    H1,
    /// `##`
    H2,
    /// `###`
    H3,
    /// `####`
    H4,
    /// `#####`
    H5,
    /// `######`
    H6,
}

/// Whether a link stays on this site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LinkKind {
    /// A link to another post or page of this site.
    Internal,
    /// A link to another site.
    External,
}

/// A link in a post's body.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LintLink {
    /// Where the link goes, as written. For an internal link, the path.
    pub target: String,
    /// The words a reader clicks. Empty when the link has none.
    pub text: String,
    /// Whether the link stays on this site.
    pub kind: LinkKind,
}

impl LintLink {
    /// A link to `target` with the clickable words `text`.
    pub fn new(kind: LinkKind, target: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            target: target.into(),
            text: text.into(),
            kind,
        }
    }
}

/// An image in a post's body.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Image {
    /// The image file or address, as written.
    pub src: String,
    /// The text that describes the image to people who cannot see it.
    pub alt: Option<String>,
}

impl Image {
    /// An image at `src` with its description, if it has one.
    pub fn new(src: impl Into<String>, alt: Option<String>) -> Self {
        Self {
            src: src.into(),
            alt,
        }
    }
}
