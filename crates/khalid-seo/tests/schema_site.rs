//! The schema every page carries: the publisher and the `WebSite`
//! (task 1.14, KH-SEO-3).

#![cfg(test)]

use chrono::NaiveDate;
use khalid_core::{AbsoluteUrl, Locale};
use khalid_seo::{Publisher, PublisherKind, SeoSite, ShareImage, SiteSearch, UrlPath, site_graph};

fn url(text: &str) -> AbsoluteUrl {
    AbsoluteUrl::parse(text).unwrap()
}

fn path(text: &str) -> UrlPath {
    UrlPath::try_new(text).unwrap()
}

/// The demo site from `fixtures/demo-site`, as far as its site nodes go.
fn demo_site() -> SeoSite {
    let mut site = SeoSite::new(
        "Field Notes on SEO",
        url("https://www.example.com"),
        Locale::try_new("en").unwrap(),
    );
    site.tagline = String::from("Plain advice on search for small teams");
    site.mount = path("/blog/");
    site
}

fn demo_company_site() -> SeoSite {
    let mut site = demo_site();
    let mut publisher = Publisher::new(PublisherKind::Organization, "SerpDad");
    publisher.logo = Some(ShareImage::new(
        url("https://www.example.com/blog/media/logo.png"),
        512,
        512,
        "The SerpDad logo",
    ));
    publisher.same_as = vec![url("https://www.linkedin.com/company/serpdad")];
    publisher.email = Some(String::from("hello@serpdad.com"));
    publisher.telephone = Some(String::from("+44 20 7946 0000"));
    publisher.founding_date = NaiveDate::from_ymd_opt(2024, 5, 1);
    site.publisher = publisher;
    site.search = SiteSearch::On(path("/search/"));
    site
}

#[test]
fn every_page_carries_the_publisher_and_the_site() {
    insta::assert_snapshot!(site_graph(&demo_company_site()).unwrap().to_json());
}

#[test]
fn a_personal_site_names_a_person_with_a_photo_and_no_founding_date() {
    let mut site = demo_company_site();
    site.publisher.kind = PublisherKind::Person;
    site.publisher.name = String::from("Asif");
    let graph = site_graph(&site).unwrap();
    let person = &graph.nodes()[0];
    assert_eq!(person.kind(), "Person");
    assert_eq!(
        person.id().map(AbsoluteUrl::as_str),
        Some("https://www.example.com/blog/#person")
    );
    assert!(person.get("image").is_some());
    assert_eq!(person.get("logo"), None);
    assert_eq!(person.get("foundingDate"), None);
}

#[test]
fn without_search_the_website_has_no_search_action() {
    let graph = site_graph(&demo_site()).unwrap();
    assert_eq!(graph.nodes()[1].kind(), "WebSite");
    assert_eq!(graph.nodes()[1].get("potentialAction"), None);
    assert_eq!(
        graph.nodes()[0].get("name"),
        Some(&khalid_seo::SchemaValue::text("Field Notes on SEO"))
    );
}
