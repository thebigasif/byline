//! What goes in a page's head: title, description, canonical, languages and share card.

use khalid_core::{AbsoluteUrl, Locale};
use serde::Serialize;

use crate::serde_display::{display, display_option};

/// The head of one page: the title tag, the description, the canonical
/// address, the other languages and the share card.
///
/// The values are worked out before rendering (task 1.13 computes them from
/// the content); templates only print them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PageMeta {
    /// The text of the `title` tag.
    pub title: String,
    /// The meta description, also used by share cards.
    pub description: String,
    /// The one address search engines should index this page under.
    #[serde(serialize_with = "display")]
    pub canonical: AbsoluteUrl,
    /// The page's path on this site, such as `/blog/what-a-sitemap-does/`.
    /// Menus mark the link with this address as the current page.
    pub path: String,
    /// What the page is, for Open Graph.
    pub kind: PageKind,
    /// Whether search engines may index the page.
    pub robots: Robots,
    /// The page's own share image. When absent, the site's default is used.
    pub image: Option<ShareImage>,
    /// This page in every language it exists in, this one included.
    pub alternates: Vec<Alternate>,
    /// The version for readers whose language the site does not have.
    #[serde(serialize_with = "display_option")]
    pub x_default: Option<AbsoluteUrl>,
}

/// What a page is, as Open Graph names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PageKind {
    /// A list, a front page or any page that is not one piece of writing.
    Website,
    /// One post or page of writing.
    Article,
}

/// Whether search engines may index a page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Robots {
    /// Index the page and follow its links.
    Index,
    /// Leave the page out of search results, such as the 404 page.
    NoIndex,
}

/// The image share cards show, with the size that keeps them from shifting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ShareImage {
    /// The image's full address.
    #[serde(serialize_with = "display")]
    pub url: AbsoluteUrl,
    /// What the image shows, for people who cannot see it.
    pub alt: String,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

/// This page in one language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Alternate {
    /// The language of that version.
    #[serde(serialize_with = "display")]
    pub locale: Locale,
    /// The version's full address.
    #[serde(serialize_with = "display")]
    pub href: AbsoluteUrl,
    /// The language's name in that language, such as `Deutsch`.
    pub label: String,
}
