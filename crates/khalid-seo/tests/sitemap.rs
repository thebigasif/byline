//! The XML sitemap of published, indexable pages (task 1.13, KH-SEO-5).

#![cfg(test)]

mod common;
mod sitemap_xsd;

use common::{at, demo_site, internal_links_post, internal_links_post_de, locale, path, url};
use khalid_seo::{
    PageKind, PublishState, SeoPage, Sitemap, SitemapListing, TranslationKey, UrlPath,
    link_translations,
};

/// The demo site's pages: four that belong in the sitemap and five that do not.
fn demo_pages() -> Vec<SeoPage> {
    let mut home = SeoPage::new(PageKind::Home, locale("en"), UrlPath::root(), "");
    home.modified = Some(at(2026, 10, 20, 9));
    let mut en = internal_links_post();
    en.translation = TranslationKey::try_new("internal-links").ok();
    let mut de = internal_links_post_de();
    de.translation = TranslationKey::try_new("internal-links").ok();
    de.published = Some(at(2026, 9, 21, 8));
    let about = SeoPage::new(PageKind::Page, locale("en"), path("/about/"), "About");

    let mut draft = SeoPage::new(
        PageKind::Post,
        locale("en"),
        path("/crawl-budget/"),
        "Crawl budget",
    );
    draft.state = PublishState::Draft;
    let mut scheduled = SeoPage::new(
        PageKind::Post,
        locale("en"),
        path("/anchor-text/"),
        "Anchor text",
    );
    scheduled.state = PublishState::Scheduled;
    let mut excluded = SeoPage::new(
        PageKind::Page,
        locale("en"),
        path("/thank-you/"),
        "Thank you",
    );
    excluded.sitemap = SitemapListing::Excluded;
    let mut thin_tag = SeoPage::new(
        PageKind::Tag,
        locale("en"),
        path("/tag/crawl-budget/"),
        "Crawl budget",
    );
    thin_tag.item_count = Some(1);
    let mut syndicated = SeoPage::new(
        PageKind::Post,
        locale("en"),
        path("/links-elsewhere/"),
        "Links",
    );
    syndicated.canonical = Some(url("https://serpdad.com/blog/links/"));
    let search = SeoPage::new(PageKind::Search, locale("en"), path("/search/"), "Search");

    vec![
        home, en, de, about, draft, scheduled, excluded, thin_tag, syndicated, search,
    ]
}

fn demo_sitemap() -> Sitemap {
    let site = demo_site();
    let mut pages = demo_pages();
    link_translations(&site, &mut pages).unwrap();
    Sitemap::from_pages(&site, &pages).unwrap()
}

#[test]
fn the_sitemap_lists_published_indexable_pages_with_alternates() {
    insta::assert_snapshot!(demo_sitemap().to_xml());
}

#[test]
fn the_sitemap_is_valid_against_the_schema() {
    let xml = demo_sitemap().to_xml();
    assert_eq!(
        sitemap_xsd::check_urlset(&xml, sitemap_xsd::google_extensions),
        Ok(4)
    );
}

#[test]
fn the_sitemap_lists_exactly_the_published_indexable_pages() {
    let locs: Vec<String> = demo_sitemap()
        .entries()
        .iter()
        .map(|entry| entry.loc.to_string())
        .collect();
    assert_eq!(
        locs,
        [
            "https://www.example.com/blog/",
            "https://www.example.com/blog/why-internal-links-matter/",
            "https://www.example.com/blog/de/warum-interne-links-wichtig-sind/",
            "https://www.example.com/blog/about/",
        ]
    );
}

#[test]
fn lastmod_is_the_change_time_else_the_publish_time_else_none() {
    let sitemap = demo_sitemap();
    let lastmod: Vec<Option<String>> = sitemap
        .entries()
        .iter()
        .map(|entry| entry.lastmod.map(|time| time.to_rfc3339()))
        .collect();
    assert_eq!(
        lastmod,
        [
            Some("2026-10-20T09:00:00+00:00".to_owned()),
            Some("2026-10-02T16:00:00+00:00".to_owned()),
            Some("2026-09-21T08:00:00+00:00".to_owned()),
            None,
        ]
    );
}

#[test]
fn an_empty_sitemap_is_still_a_valid_document() {
    let sitemap = Sitemap::from_pages(&demo_site(), &[]).unwrap();
    assert_eq!(
        sitemap_xsd::check_urlset(&sitemap.to_xml(), sitemap_xsd::google_extensions),
        Ok(0)
    );
}
