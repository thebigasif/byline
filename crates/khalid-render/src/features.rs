//! The reader features a site shows, each one switched off in `site.toml`.

use std::collections::BTreeSet;

use serde::Serialize;

/// One reader feature of the default templates (plan section 4.12).
///
/// Templates test for one by its name, such as
/// `{% if "reading_time" in site.features %}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Feature {
    /// Each author's photo, name and job title above a post.
    Byline,
    /// "Reviewed by" in the byline.
    ReviewedBy,
    /// "8 min read" next to a post's date.
    ReadingTime,
    /// The table of contents of a long post.
    TableOfContents,
    /// A link to each H2 and H3 heading of a post.
    HeadingLinks,
    /// A link from a body image to its largest version.
    EnlargeImages,
    /// The authors' photos, roles and bios at the end of a post.
    AuthorBox,
    /// The related posts under a post.
    RelatedPosts,
    /// The author, the date and the category on every post card.
    RichCards,
    /// The row of links to every category on the home page.
    TopicLinks,
    /// The page of every author, and the other authors on each author page.
    AuthorsPage,
    /// The page of every post across collections.
    Archive,
    /// The search link in the header.
    SearchShortcut,
    /// The list of posts that link to a post, under it.
    LinkedMentions,
    /// The title and summary of a linked post, shown on pointing at its link.
    LinkPreviews,
}

impl Feature {
    /// Every feature, in the order the plan lists them.
    pub const ALL: [Self; 15] = [
        Self::Byline,
        Self::ReviewedBy,
        Self::ReadingTime,
        Self::TableOfContents,
        Self::HeadingLinks,
        Self::EnlargeImages,
        Self::AuthorBox,
        Self::RelatedPosts,
        Self::RichCards,
        Self::TopicLinks,
        Self::AuthorsPage,
        Self::Archive,
        Self::SearchShortcut,
        Self::LinkedMentions,
        Self::LinkPreviews,
    ];
}

/// The reader features that are on. Templates read it as a list of names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Features(BTreeSet<Feature>);

impl Features {
    /// Every feature on, the default of a new site.
    pub fn all() -> Self {
        Self(Feature::ALL.into_iter().collect())
    }

    /// These features with `feature` switched off.
    #[must_use]
    pub fn without(mut self, feature: Feature) -> Self {
        self.0.remove(&feature);
        self
    }

    /// Whether `feature` is on.
    pub fn contains(&self, feature: Feature) -> bool {
        self.0.contains(&feature)
    }
}
