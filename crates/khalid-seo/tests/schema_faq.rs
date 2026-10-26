//! The FAQ block (task 1.14, KH-SEO-3).

#![cfg(test)]

mod common;

use common::{demo_site, internal_links_post, internal_links_post_de};
use khalid_seo::{FaqItem, SchemaBlock, SchemaGraph, SchemaNode, page_graph};

fn kinds(graph: &SchemaGraph) -> Vec<&str> {
    graph.nodes().iter().map(SchemaNode::kind).collect()
}

fn faq() -> SchemaBlock {
    SchemaBlock::Faq(vec![
        FaqItem::new(
            "How many internal links should a post have?",
            "Enough to help the reader. Three to five is a good start.",
        ),
        FaqItem::new(
            "Do internal links pass ranking signals?",
            "Yes. They tell search engines which pages matter most.",
        ),
    ])
}

#[test]
fn a_post_with_an_faq_block_adds_an_faq_page() {
    let mut post = internal_links_post();
    post.blocks = vec![faq()];
    let graph = page_graph(&demo_site(), &post).unwrap();
    assert_eq!(kinds(&graph).last(), Some(&"FAQPage"));
    let node = graph.nodes().last().unwrap().clone();
    insta::assert_snapshot!(SchemaGraph::new(vec![node]).to_json());
}

#[test]
fn an_empty_faq_writes_nothing() {
    let mut post = internal_links_post_de();
    post.blocks = vec![SchemaBlock::Faq(Vec::new())];
    let graph = page_graph(&demo_site(), &post).unwrap();
    assert!(!kinds(&graph).contains(&"FAQPage"));
}
