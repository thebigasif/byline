//! The sitemap index (task 1.13, KH-SEO-5).

#![cfg(test)]

mod common;
mod sitemap_index_xsd;
mod sitemap_xsd;

use common::{at, demo_site, internal_links_post, internal_links_post_de, url};
use khalid_seo::{Sitemap, SitemapIndex, sitemap_index_url};
use sitemap_index_xsd::check_sitemapindex;
use sitemap_xsd::{check_urlset, google_extensions};

/// An index of the English and German post sitemaps and the news sitemap.
fn demo_index() -> SitemapIndex {
    let site = demo_site();
    let english = Sitemap::from_pages(&site, &[internal_links_post()]).unwrap();
    let german = Sitemap::from_pages(&site, &[internal_links_post_de()]).unwrap();
    for sitemap in [&english, &german] {
        assert!(check_urlset(&sitemap.to_xml(), google_extensions).is_ok());
    }
    let mut index = SitemapIndex::default();
    let latest = english
        .entries()
        .iter()
        .filter_map(|entry| entry.lastmod)
        .max();
    index.push(
        url("https://www.example.com/blog/sitemaps/posts-en-1.xml"),
        latest,
    );
    index.push(
        url("https://www.example.com/blog/sitemaps/posts-de-1.xml"),
        None,
    );
    index.push(
        url("https://www.example.com/blog/sitemaps/news.xml"),
        Some(at(2026, 10, 25, 7)),
    );
    index
}

#[test]
fn the_index_lists_every_file_with_its_latest_change() {
    insta::assert_snapshot!(demo_index().to_xml());
}

#[test]
fn the_index_is_valid_against_the_schema() {
    assert_eq!(check_sitemapindex(&demo_index().to_xml()), Ok(3));
}

#[test]
fn an_empty_index_is_still_a_valid_document() {
    assert_eq!(check_sitemapindex(&SitemapIndex::default().to_xml()), Ok(0));
}

#[test]
fn the_index_lives_in_the_mount_folder() {
    assert_eq!(
        sitemap_index_url(&demo_site()).unwrap().as_str(),
        "https://www.example.com/blog/sitemap.xml"
    );
}

#[test]
fn the_index_checks_refuse_what_siteindex_refuses() {
    let head = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";
    let ns = "http://www.sitemaps.org/schemas/sitemap/0.9";
    for bad in [
        "<sitemap><lastmod>2026-10-02</lastmod></sitemap>",
        "<sitemap><loc>/relative.xml</loc></sitemap>",
        "<sitemap><lastmod>2026-10-02</lastmod><loc>https://example.com/a.xml</loc></sitemap>",
        "<url><loc>https://example.com/a.xml</loc></url>",
    ] {
        let xml = format!("{head}<sitemapindex xmlns=\"{ns}\">{bad}</sitemapindex>");
        assert!(check_sitemapindex(&xml).is_err(), "{bad}");
    }
}
