//! A local business and its locations (task 1.14, KH-SEO-3).

#![cfg(test)]

mod common;

use chrono::{NaiveTime, Weekday};
use common::{demo_site, internal_links_post, internal_links_post_de, locale, url};
use khalid_seo::{
    LocalBusiness, OpeningHours, PageKind, PostalAddress, SchemaGraph, SchemaNode, SeoPage,
    UrlPath, page_graph,
};

fn kinds(graph: &SchemaGraph) -> Vec<&str> {
    graph.nodes().iter().map(SchemaNode::kind).collect()
}

fn dentist() -> LocalBusiness {
    let mut business = LocalBusiness::new(
        "Dentist",
        "Riverside Dental",
        PostalAddress::new("12 Mill Lane", "Bristol", "BS1 4DJ", "GB"),
    );
    business.telephone = Some(String::from("+44 117 496 0000"));
    business.price_range = Some(String::from("££"));
    business.map = Some(url("https://maps.example.com/riverside-dental"));
    business.opening_hours = vec![OpeningHours::new(
        vec![
            Weekday::Mon,
            Weekday::Tue,
            Weekday::Wed,
            Weekday::Thu,
            Weekday::Fri,
        ],
        NaiveTime::from_hms_opt(8, 30, 0).unwrap(),
        NaiveTime::from_hms_opt(17, 0, 0).unwrap(),
    )];
    business
}

fn home() -> SeoPage {
    SeoPage::new(PageKind::Home, locale("en"), UrlPath::root(), "")
}

#[test]
fn the_home_page_names_each_location() {
    let mut site = demo_site();
    site.locations = vec![dentist()];
    let graph = page_graph(&site, &home()).unwrap();
    let business = graph.nodes().last().unwrap().clone();
    insta::assert_snapshot!(SchemaGraph::new(vec![business]).to_json());
}

#[test]
fn several_locations_get_one_node_each_and_posts_get_none() {
    let mut site = demo_site();
    let mut second = dentist();
    second.name = String::from("Riverside Dental Clifton");
    second.address.region = Some(String::from("Bristol"));
    site.locations = vec![dentist(), second];
    let graph = page_graph(&site, &home()).unwrap();
    let dentists: Vec<&SchemaNode> = graph
        .nodes()
        .iter()
        .filter(|n| n.kind() == "Dentist")
        .collect();
    assert_eq!(dentists.len(), 2);
    assert!(dentists[1].id().unwrap().as_str().ends_with("#location-2"));
    for post in [internal_links_post(), internal_links_post_de()] {
        let graph = page_graph(&site, &post).unwrap();
        assert!(!kinds(&graph).contains(&"Dentist"));
    }
}

#[test]
fn a_site_that_is_not_a_local_business_names_no_location() {
    let graph = page_graph(&demo_site(), &home()).unwrap();
    assert_eq!(kinds(&graph).last(), Some(&"WebPage"));
}
