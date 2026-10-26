//! Glossary terms as defined terms (task 1.14, KH-SEO-3).

#![cfg(test)]

mod common;

use common::{demo_site, internal_links_post, internal_links_post_de, locale, path};
use khalid_seo::{PageKind, SchemaGraph, SchemaNode, SeoPage, page_graph};

fn kinds(graph: &SchemaGraph) -> Vec<&str> {
    graph.nodes().iter().map(SchemaNode::kind).collect()
}

fn term() -> SeoPage {
    let mut page = SeoPage::new(
        PageKind::GlossaryTerm,
        locale("en"),
        path("/glossary/anchor-text/"),
        "Anchor text",
    );
    page.excerpt = String::from("The words a reader clicks to follow a link.");
    page
}

#[test]
fn a_glossary_term_is_a_defined_term_in_the_glossary_set() {
    let graph = page_graph(&demo_site(), &term()).unwrap();
    assert_eq!(kinds(&graph).last(), Some(&"DefinedTerm"));
    let node = graph.nodes().last().unwrap().clone();
    insta::assert_snapshot!(SchemaGraph::new(vec![node]).to_json());
}

#[test]
fn the_glossary_index_is_the_defined_term_set() {
    let index = SeoPage::new(
        PageKind::GlossaryIndex,
        locale("en"),
        path("/glossary/"),
        "Glossary",
    );
    let graph = page_graph(&demo_site(), &index).unwrap();
    let set = graph.nodes().last().unwrap();
    assert_eq!(set.kind(), "DefinedTermSet");
    assert_eq!(
        set.id().map(khalid_core::AbsoluteUrl::as_str),
        Some("https://www.example.com/blog/glossary/#termset")
    );
}

#[test]
fn a_german_term_points_to_the_german_glossary() {
    let mut german = term();
    german.language = locale("de");
    let graph = page_graph(&demo_site(), &german).unwrap();
    let json: serde_json::Value = serde_json::from_str(&graph.to_json()).unwrap();
    let last = json["@graph"].as_array().unwrap().last().unwrap().clone();
    assert_eq!(
        last["inDefinedTermSet"]["@id"],
        "https://www.example.com/blog/de/glossary/#termset"
    );
}

#[test]
fn posts_carry_no_glossary_nodes() {
    for post in [internal_links_post(), internal_links_post_de()] {
        let graph = page_graph(&demo_site(), &post).unwrap();
        assert!(!kinds(&graph).contains(&"DefinedTerm"));
    }
}
