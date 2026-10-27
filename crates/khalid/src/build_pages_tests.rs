//! The pages `khalid build` renders, checked on the demo site.

use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use khalid_content::{ContentTree, read_content};
use khalid_core::{Config, Environment};
use khalid_routes::Routes;
use khalid_seo::{Feed, FeedScope};

use crate::build_seo;

fn demo() -> PathBuf {
    let mut path = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
    path.push("../../fixtures/demo-site");
    path
}

#[test]
fn the_english_feed_carries_each_wikilink_as_a_full_address() {
    let site = demo();
    let config = Config::load(&site, &Environment::empty()).unwrap();
    let routes = Routes::load(&site).unwrap();
    let content = read_content(&ContentTree::walk(&site).unwrap()).unwrap();
    let stylesheet = "/blog/assets/site.css";
    let mut languages = super::pages(&config, &routes, (&content, &[]), stylesheet).unwrap();
    let seo_site = build_seo::site(&config).unwrap();
    let seo_pages = build_seo::heads(&seo_site, &mut languages).unwrap();
    let english = FeedScope::language(config.default_language().clone());
    let now = build_seo::midnight(NaiveDate::default());
    let rss = Feed::for_pages(&seo_site, &seo_pages, &english, now)
        .unwrap()
        .to_rss()
        .unwrap();
    let link = "https://www.example.com/blog/glossary/topical-authority/";
    assert_eq!(rss.matches(link).count(), 2, "{rss}");
    assert!(rss.contains("Topical authority&lt;/a&gt;"), "{rss}");
    assert!(!rss.contains("topical-authority."), "{rss}");
}

#[test]
fn only_addresses_from_the_site_root_become_full() {
    let config = Config::load(&demo(), &Environment::empty()).unwrap();
    let html = "<a href=\"/blog/a/\">a</a><img src=\"/blog/b.png\" alt=\"\">\
                <a href=\"//cdn.example.org/c\">c</a><a href=\"https://x.org/\">x</a>\
                <a title=\"/t\" href=\"#d\">d</a><code>href=&quot;/e&quot;</code>";
    assert_eq!(
        super::full_addresses(&config, html),
        "<a href=\"https://www.example.com/blog/a/\">a</a>\
         <img src=\"https://www.example.com/blog/b.png\" alt=\"\">\
         <a href=\"//cdn.example.org/c\">c</a><a href=\"https://x.org/\">x</a>\
         <a title=\"/t\" href=\"#d\">d</a><code>href=&quot;/e&quot;</code>"
    );
}
