//! The demo site from `fixtures/demo-site`, as the SEO crate sees it.
//!
//! Task 1.15 wires the real content in. Until then these helpers mirror
//! `fixtures/demo-site/site.toml` and a few of its posts by hand.

#![cfg(test)]

use khalid_core::{AbsoluteUrl, Locale};
use khalid_seo::{PageKind, SeoPage, SeoSite, UrlPath};

pub(crate) fn locale(tag: &str) -> Locale {
    Locale::try_new(tag).unwrap()
}

pub(crate) fn path(text: &str) -> UrlPath {
    UrlPath::try_new(text).unwrap()
}

pub(crate) fn url(text: &str) -> AbsoluteUrl {
    AbsoluteUrl::parse(text).unwrap()
}

/// The demo site: English by default, German as the second language, under `/blog/`.
pub(crate) fn demo_site() -> SeoSite {
    let mut site = SeoSite::new(
        "Field Notes on SEO",
        url("https://www.example.com"),
        locale("en"),
    );
    site.tagline = String::from("Plain advice on search for small teams");
    site.mount = path("/blog/");
    site.languages = vec![locale("en"), locale("de")];
    site
}

/// The English post "Why internal links matter".
pub(crate) fn internal_links_post() -> SeoPage {
    let mut page = SeoPage::new(
        PageKind::Post,
        locale("en"),
        path("/why-internal-links-matter/"),
        "Why internal links matter",
    );
    page.excerpt = String::from(
        "Internal links help readers and search engines find the rest of your \
         site. Here is how to add them without overthinking it.",
    );
    page.category = Some(String::from("On-page SEO"));
    page.author = Some(String::from("Asif"));
    page
}

/// The German translation of the same post.
pub(crate) fn internal_links_post_de() -> SeoPage {
    let mut page = SeoPage::new(
        PageKind::Post,
        locale("de"),
        path("/warum-interne-links-wichtig-sind/"),
        "Warum interne Links wichtig sind",
    );
    page.excerpt = String::from(
        "Interne Links helfen Lesern und Suchmaschinen, den Rest Ihrer Website zu finden.",
    );
    page.author = Some(String::from("Asif"));
    page
}
