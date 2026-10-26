//! The HTML sitemap page's model (task 1.13, KH-SEO-5).

#![cfg(test)]

mod common;

use std::fmt::Write as _;

use common::{demo_site, internal_links_post, internal_links_post_de, locale, path};
use khalid_seo::{HtmlSitemap, PageKind, PublishState, SeoPage, UrlPath};

fn demo_pages() -> Vec<SeoPage> {
    let home = SeoPage::new(PageKind::Home, locale("en"), UrlPath::root(), "");
    let about = SeoPage::new(PageKind::Page, locale("en"), path("/about/"), "About");
    let anchor = SeoPage::new(
        PageKind::Post,
        locale("en"),
        path("/anchor-text/"),
        "anchor text, plainly",
    );
    let mut draft = SeoPage::new(PageKind::Post, locale("en"), path("/draft/"), "A draft");
    draft.state = PublishState::Draft;
    let mut tag = SeoPage::new(PageKind::Tag, locale("en"), path("/tag/links/"), "Links");
    tag.item_count = Some(5);
    vec![
        internal_links_post(),
        internal_links_post_de(),
        home,
        about,
        anchor,
        draft,
        tag,
    ]
}

fn outline(sitemap: &HtmlSitemap) -> String {
    let mut out = String::new();
    for group in &sitemap.groups {
        writeln!(out, "{:?}", group.section).unwrap();
        for link in &group.links {
            writeln!(out, "  {} <{}>", link.title, link.url).unwrap();
        }
    }
    out
}

#[test]
fn the_english_sitemap_groups_listed_pages_by_section_and_title() {
    let sitemap = HtmlSitemap::from_pages(&demo_site(), &demo_pages(), &locale("en")).unwrap();
    insta::assert_snapshot!(outline(&sitemap));
}

#[test]
fn each_language_gets_only_its_own_pages() {
    let sitemap = HtmlSitemap::from_pages(&demo_site(), &demo_pages(), &locale("de")).unwrap();
    assert_eq!(sitemap.language, locale("de"));
    let titles: Vec<&str> = sitemap
        .groups
        .iter()
        .flat_map(|group| group.links.iter().map(|link| link.title.as_str()))
        .collect();
    assert_eq!(titles, ["Warum interne Links wichtig sind"]);
}

#[test]
fn a_language_with_nothing_published_has_no_groups() {
    let sitemap = HtmlSitemap::from_pages(&demo_site(), &demo_pages(), &locale("fr")).unwrap();
    assert_eq!(sitemap.groups, []);
}
