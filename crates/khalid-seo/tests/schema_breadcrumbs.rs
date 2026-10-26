//! `BreadcrumbList` on every page (task 1.14, KH-SEO-3).

#![cfg(test)]

mod common;

use common::{demo_site, internal_links_post, internal_links_post_de, locale, path};
use khalid_seo::{Crumb, PageKind, SchemaGraph, SeoPage, UrlPath, breadcrumb_list};

fn items(page: &SeoPage) -> Vec<serde_json::Value> {
    let node = breadcrumb_list(&demo_site(), page).unwrap();
    let json: serde_json::Value =
        serde_json::from_str(&SchemaGraph::new(vec![node]).to_json()).unwrap();
    json["@graph"][0]["itemListElement"]
        .as_array()
        .unwrap()
        .clone()
}

#[test]
fn a_post_in_a_category_has_home_category_and_post() {
    let mut post = internal_links_post();
    post.breadcrumbs = vec![Crumb::new("On-page SEO", path("/category/on-page-seo/"))];
    let node = breadcrumb_list(&demo_site(), &post).unwrap();
    insta::assert_snapshot!(SchemaGraph::new(vec![node]).to_json());
}

#[test]
fn the_home_page_breadcrumb_is_the_home_page_alone() {
    let home = SeoPage::new(PageKind::Home, locale("en"), UrlPath::root(), "");
    let items = items(&home);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["name"], "Field Notes on SEO");
    assert_eq!(items[0]["position"], 1);
}

#[test]
fn a_german_page_starts_at_the_german_home_page() {
    let items = items(&internal_links_post_de());
    assert_eq!(items[0]["item"], "https://www.example.com/blog/de/");
    assert_eq!(items[1]["position"], 2);
    assert_eq!(
        items[1]["item"],
        "https://www.example.com/blog/de/warum-interne-links-wichtig-sind/"
    );
}
