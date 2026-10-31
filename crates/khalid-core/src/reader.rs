//! The reader features of the default templates, from the `[reader]` table
//! of `site.toml` (plan section 4.12).

use serde::Deserialize;

use crate::Slug;

/// Which reader features the default templates show, and the home page's
/// sections. Every feature is on unless `site.toml` switches it off:
///
/// ```toml
/// [reader]
/// table_of_contents = false
///
/// [[reader.home_sections]]
/// category = "technical-seo"
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[non_exhaustive]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each field is one independent on or off key of site.toml, and every combination is valid"
)]
pub struct Reader {
    /// Each author's photo, name and job title above a post.
    pub byline: bool,
    /// "Reviewed by" in the byline, and `reviewedBy` in the page's JSON-LD.
    pub reviewed_by: bool,
    /// "8 min read" next to a post's date.
    pub reading_time: bool,
    /// The table of contents of a post with three or more H2 headings.
    pub table_of_contents: bool,
    /// A link to each H2 and H3 heading of a post.
    pub heading_links: bool,
    /// A link from a body image to its largest version.
    pub enlarge_images: bool,
    /// The authors' photos, roles and bios at the end of a post.
    pub author_box: bool,
    /// Three to five related posts under a post.
    pub related_posts: bool,
    /// The author, the date and the category on every post card.
    pub rich_cards: bool,
    /// A row of links to every category on the home page.
    pub topic_links: bool,
    /// The `/authors/` page, and the other authors on each author page.
    pub authors_page: bool,
    /// The `/archive/` page of every post.
    pub archive: bool,
    /// The search link in the header, and the `/` and Ctrl+K keys.
    pub search_shortcut: bool,
    /// The list of the site's posts that link to a post, under it.
    pub linked_mentions: bool,
    /// The title and summary of a linked post, shown when a reader points
    /// at or focuses a link to it in a body.
    pub link_previews: bool,
    /// The sections of the home page, in order. Empty means one list of the
    /// newest posts.
    pub home_sections: Vec<HomeSection>,
}

impl Default for Reader {
    fn default() -> Self {
        Self {
            byline: true,
            reviewed_by: true,
            reading_time: true,
            table_of_contents: true,
            heading_links: true,
            enlarge_images: true,
            author_box: true,
            related_posts: true,
            rich_cards: true,
            topic_links: true,
            authors_page: true,
            archive: true,
            search_shortcut: true,
            linked_mentions: true,
            link_previews: true,
            home_sections: Vec::new(),
        }
    }
}

/// One section of the home page: the newest posts of a category, a
/// collection or a channel, with a "View all" link to its own page.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum HomeSection {
    /// A category from `categories.toml`, by slug.
    Category(Slug),
    /// A collection from `routes.toml`, such as `guides`.
    Collection(Slug),
    /// A channel from `routes.toml`, such as `featured`.
    Channel(Slug),
}
