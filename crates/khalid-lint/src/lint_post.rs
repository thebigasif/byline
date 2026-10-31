//! One post as the linter sees it.

use chrono::NaiveDate;
use khalid_core::{Locale, Slug};
use serde::Deserialize;

/// A post, reduced to what the rules read.
///
/// The content track builds it from a parsed and rendered post; `SerpDad` builds
/// it from a draft. Fields that can be missing in a draft are `Option`, so a
/// rule sees the gap instead of a made-up value. Start from [`LintPost::new`]
/// and set the fields you have, or read one from JSON or TOML with `serde`,
/// where every field but `locale` and `slug` may be left out.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct LintPost {
    /// The language the post is written in.
    pub locale: Locale,
    /// The last part of the post's address.
    pub slug: Slug,
    /// The post's path on the site, such as `/blog/why-links-matter/`, when
    /// the caller knows it. Links then find the post by path; without it,
    /// by the last part of the link matching the slug.
    pub path: Option<String>,
    /// The title from the header, which becomes the page's main headline.
    pub title: Option<String>,
    /// The title search results show in place of the headline, from
    /// `seo_title` in the header, as written, `{site}` and `{sep}` included.
    pub seo_title: Option<String>,
    /// The description from the header, which search results show under the
    /// title.
    pub description: Option<String>,
    /// The publish date from the header, written `2026-10-26` in JSON or TOML.
    pub date: Option<NaiveDate>,
    /// The date of the last real update, from the header.
    pub updated: Option<NaiveDate>,
    /// The author from the header.
    pub author: Option<String>,
    /// The `translation` key from the header, shared by every language
    /// version of the same post.
    pub translation: Option<String>,
    /// The topic group the post belongs to, and its role in it.
    pub cluster: Option<Cluster>,
    /// The headings in the body, in order. The title is not one of them.
    #[serde(default)]
    pub headings: Vec<Heading>,
    /// The links in the body, in order.
    #[serde(default)]
    pub links: Vec<LintLink>,
    /// The images in the body, in order.
    #[serde(default)]
    pub images: Vec<Image>,
    /// The targets of the `[[slug]]` links in the body, as written, in order.
    #[serde(default)]
    pub wikilinks: Vec<String>,
    /// How many words the body holds.
    #[serde(default)]
    pub word_count: usize,
    /// The body as plain text, with markup removed, one line per paragraph,
    /// heading or list item, so a rule can find the first paragraph.
    #[serde(default)]
    pub body_text: String,
    /// Old paths that redirect to this post, such as `/2019/05/old-title/`,
    /// from `aliases` in the header.
    #[serde(default)]
    pub aliases: Vec<String>,
    /// The forms in the body, in order.
    #[serde(default)]
    pub forms: Vec<LintForm>,
    /// How many tables the body holds.
    #[serde(default)]
    pub tables: usize,
    /// How many review blocks, each a rated verdict on one thing, the body
    /// holds.
    #[serde(default)]
    pub reviews: usize,
    /// The phrase the post wants to be found for, such as "internal links",
    /// from `focus_keyword` in the header.
    pub focus_keyword: Option<String>,
    /// Other phrases close to the focus keyword that the post covers, such
    /// as "link text", from `secondary_keywords` in the header.
    #[serde(default)]
    pub secondary_keywords: Vec<String>,
    /// The collection the post belongs to, such as `guides`, from the
    /// folder it lives in.
    pub collection: Option<String>,
    /// The properties Google needs that the page's structured data lacks,
    /// as `khalid-seo`'s check of the page's graph reports them.
    #[serde(default)]
    pub structured_data_gaps: Vec<StructuredDataGap>,
}

impl LintPost {
    /// An empty post with only its language and address set.
    pub fn new(locale: Locale, slug: Slug) -> Self {
        Self {
            locale,
            slug,
            path: None,
            title: None,
            seo_title: None,
            description: None,
            date: None,
            updated: None,
            author: None,
            translation: None,
            cluster: None,
            headings: Vec::new(),
            links: Vec::new(),
            images: Vec::new(),
            wikilinks: Vec::new(),
            word_count: 0,
            body_text: String::new(),
            aliases: Vec::new(),
            forms: Vec::new(),
            tables: 0,
            reviews: 0,
            focus_keyword: None,
            secondary_keywords: Vec::new(),
            collection: None,
            structured_data_gaps: Vec::new(),
        }
    }
}

/// A property Google needs that a node of the page's structured data does
/// not have.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct StructuredDataGap {
    /// The type of the node, such as `Event`.
    pub kind: String,
    /// The missing property, or the choice of them, such as `startDate`.
    pub property: String,
}

impl StructuredDataGap {
    /// The node of type `kind` lacks `property`.
    pub fn new(kind: impl Into<String>, property: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            property: property.into(),
        }
    }
}

/// A form in a post's body, and where its answers go.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct LintForm {
    /// The name the form's answers are kept under.
    pub id: String,
    /// Where each answer is delivered. Empty when the site sends them
    /// nowhere.
    #[serde(default)]
    pub destinations: Vec<FormDestination>,
}

impl LintForm {
    /// The form `id`, delivering to `destinations`.
    pub fn new(id: impl Into<String>, destinations: Vec<FormDestination>) -> Self {
        Self {
            id: id.into(),
            destinations,
        }
    }
}

/// A way a form's answers reach a person, written `email` or `webhook`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum FormDestination {
    /// An email to an address the site sets.
    Email,
    /// A request to an address another service listens on, such as Zapier.
    Webhook,
}

/// A topic group, often called a cluster: one main post, the pillar, and the
/// posts around it that link to it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct Cluster {
    /// The group's name, the same on every post in it.
    pub name: String,
    /// Whether this post is the group's main post.
    pub role: ClusterRole,
}

impl Cluster {
    /// A place in the group `name` with `role`.
    pub fn new(name: impl Into<String>, role: ClusterRole) -> Self {
        Self {
            name: name.into(),
            role,
        }
    }
}

/// A post's role in its topic group, written `pillar` or `member`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClusterRole {
    /// The group's main post, which covers the topic as a whole.
    Pillar,
    /// A post on one part of the topic, which links to the pillar.
    Member,
}

/// A heading in a post's body.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
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

/// The six heading levels of HTML, from `h1` to `h6`, written that way in
/// JSON or TOML.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
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

impl HeadingLevel {
    /// The level as a number from 1 to 6, as in `h1` to `h6`.
    pub const fn number(self) -> u8 {
        match self {
            Self::H1 => 1,
            Self::H2 => 2,
            Self::H3 => 3,
            Self::H4 => 4,
            Self::H5 => 5,
            Self::H6 => 6,
        }
    }
}

/// Whether a link stays on this site, written `internal` or `external` in
/// JSON or TOML.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LinkKind {
    /// A link to another post or page of this site.
    Internal,
    /// A link to another site.
    External,
}

/// A link in a post's body.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[non_exhaustive]
pub struct LintLink {
    /// Where the link goes, as written. For an internal link, the path.
    pub target: String,
    /// The words a reader clicks. Empty when the link has none.
    #[serde(default)]
    pub text: String,
    /// Whether the link stays on this site.
    pub kind: LinkKind,
    /// The words of the link's `rel` attribute, such as `nofollow`, as
    /// written. Empty when the link has none, as markdown links do.
    #[serde(default)]
    pub rel: Vec<String>,
}

impl LintLink {
    /// A link to `target` with the clickable words `text`.
    pub fn new(kind: LinkKind, target: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            target: target.into(),
            text: text.into(),
            kind,
            rel: Vec::new(),
        }
    }
}

/// An image in a post's body.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
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
