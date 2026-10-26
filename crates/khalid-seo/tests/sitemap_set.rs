//! Sitemap files per section and language, split at 50,000, and the
//! index of them (task 1.13, KH-SEO-5).

#![cfg(test)]

mod common;
mod sitemap_index_xsd;
mod sitemap_xsd;

use common::{demo_site, internal_links_post, internal_links_post_de, locale, path};
use khalid_seo::{PageKind, SeoPage, SitemapSet, UrlPath, link_translations};
use sitemap_index_xsd::check_sitemapindex;
use sitemap_xsd::{check_urlset, google_extensions};

fn demo_pages() -> Vec<SeoPage> {
    let home = SeoPage::new(PageKind::Home, locale("en"), UrlPath::root(), "");
    let about = SeoPage::new(PageKind::Page, locale("en"), path("/about/"), "About");
    let mut tag = SeoPage::new(PageKind::Tag, locale("en"), path("/tag/links/"), "Links");
    tag.item_count = Some(4);
    vec![
        home,
        internal_links_post(),
        internal_links_post_de(),
        about,
        tag,
    ]
}

fn demo_set() -> SitemapSet {
    let site = demo_site();
    let mut pages = demo_pages();
    link_translations(&site, &mut pages).unwrap();
    SitemapSet::from_pages(&site, &pages).unwrap()
}

#[test]
fn each_section_and_language_gets_its_own_file() {
    let paths: Vec<String> = demo_set()
        .files()
        .iter()
        .map(|file| file.path.to_string())
        .collect();
    assert_eq!(
        paths,
        [
            "/sitemaps/posts-en-1.xml",
            "/sitemaps/posts-de-1.xml",
            "/sitemaps/pages-en-1.xml",
            "/sitemaps/archives-en-1.xml",
        ]
    );
}

#[test]
fn the_index_lists_every_file_of_the_set() {
    insta::assert_snapshot!(demo_set().index().to_xml());
}

#[test]
fn the_index_and_every_file_are_valid_against_the_schemas() {
    let set = demo_set();
    assert_eq!(check_sitemapindex(&set.index().to_xml()), Ok(4));
    for file in set.files() {
        assert!(
            check_urlset(&file.sitemap.to_xml(), google_extensions).is_ok(),
            "{}",
            file.path
        );
    }
}

#[test]
fn a_file_is_split_at_fifty_thousand_addresses() {
    let site = demo_site();
    let pages: Vec<SeoPage> = (0..50_001)
        .map(|n| {
            SeoPage::new(
                PageKind::Post,
                locale("en"),
                path(&format!("/note-{n}/")),
                "Note",
            )
        })
        .collect();
    let set = SitemapSet::from_pages(&site, &pages).unwrap();
    let sizes: Vec<(String, usize)> = set
        .files()
        .iter()
        .map(|file| (file.path.to_string(), file.sitemap.entries().len()))
        .collect();
    assert_eq!(
        sizes,
        [
            ("/sitemaps/posts-en-1.xml".to_owned(), 50_000),
            ("/sitemaps/posts-en-2.xml".to_owned(), 1),
        ]
    );
    let first = set.files()[0].sitemap.to_xml();
    assert_eq!(check_urlset(&first, google_extensions), Ok(50_000));
}

#[test]
fn a_site_with_nothing_published_has_an_empty_valid_index() {
    let set = SitemapSet::from_pages(&demo_site(), &[]).unwrap();
    assert_eq!(set.files().len(), 0);
    assert_eq!(check_sitemapindex(&set.index().to_xml()), Ok(0));
}
