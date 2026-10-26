//! The site as one page sees it: its name, menus and stylesheet, in one language.

use khalid_core::Locale;
use serde::Serialize;

use crate::Features;
use crate::page_meta::ShareImage;

/// The site in one language, the same for every page in that language.
///
/// The wiring code builds one per language from the settings and the theme.
/// Fields are public and the struct is exhaustive on purpose: a new field
/// must stop the build until the wiring fills it (ADR 0009).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RenderSite {
    /// The site's name, shown in the header and in share cards.
    pub name: String,
    /// The language every page rendered with this value is written in.
    #[serde(skip)]
    pub locale: Locale,
    /// The address of the blog's front page in this language, such as `/blog/`.
    pub home: String,
    /// The address of the hashed stylesheet, such as `/assets/site.3f2a9c1e.css`.
    pub stylesheet: String,
    /// The menus.
    pub nav: Nav,
    /// The link back to the main website, when the blog lives beside one.
    pub main_site: Option<NavLink>,
    /// The share image for pages that have none of their own.
    pub share_image: Option<ShareImage>,
    /// The reader features that are on, from `[reader]` in `site.toml`.
    pub features: Features,
    /// The address of the search page, such as `/blog/search/`, which the
    /// header links to. `None` when the site has no search page.
    pub search: Option<String>,
}

/// The site's menus. An empty menu renders nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Nav {
    /// The main menu in the header.
    pub primary: Vec<NavLink>,
    /// The menu in the footer.
    pub footer: Vec<NavLink>,
}

/// One link in a menu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NavLink {
    /// The words a reader sees.
    pub label: String,
    /// Where it goes: a path on this site, such as `/blog/about/`, or a full address.
    pub href: String,
}
