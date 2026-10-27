//! The search link in the header of every page: a plain link to the search
//! page, which works with scripts off.

use khalid_render::Feature;

use crate::html_check::assert_valid_page;
use crate::post::{render, render_on, site_without, sitemap_post};

#[test]
fn every_page_links_to_search_from_its_header_in_its_language() {
    let english = render("en", sitemap_post());
    assert_valid_page(&english);
    assert!(
        english.contains(
            "</nav>\n<a class=\"search-link\" href=\"/blog/search/\">Search</a>\n</header>"
        )
    );
    let german = render("de", sitemap_post());
    assert!(german.contains("<a class=\"search-link\" href=\"/blog/de/search/\">Suche</a>"));
}

#[test]
fn switched_off_or_without_a_search_page_there_is_no_link() {
    let off = render_on(&site_without(Feature::SearchShortcut), sitemap_post());
    assert_valid_page(&off);
    assert!(!off.contains("search-link"));
    let mut site = crate::common::demo_site("en");
    site.search = None;
    assert!(!render_on(&site, sitemap_post()).contains("search-link"));
}
