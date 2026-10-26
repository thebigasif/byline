//! The how-to block and money (task 1.14, KH-SEO-3).

#![cfg(test)]

mod common;

use std::time::Duration;

use common::{demo_site, internal_links_post, internal_links_post_de};
use khalid_seo::{HowTo, HowToStep, Money, SchemaBlock, SchemaGraph, SeoError, page_graph};

fn how_to() -> HowTo {
    let mut how_to = HowTo::new(
        "Add internal links to an old post",
        vec![
            HowToStep::new("Find the post", "Open the post that gets the most visits."),
            HowToStep::new("Pick three newer posts", "Choose posts on the same topic."),
        ],
    );
    how_to.total_time = Some(Duration::from_mins(15));
    how_to.tools = vec![String::from("A site search")];
    how_to.supplies = vec![String::from("A list of your posts")];
    how_to.estimated_cost = Some(Money::new(0, "GBP").unwrap());
    how_to
}

#[test]
fn a_post_with_a_how_to_block_adds_a_how_to() {
    let mut post = internal_links_post();
    post.blocks = vec![SchemaBlock::HowTo(how_to())];
    let graph = page_graph(&demo_site(), &post).unwrap();
    let node = graph.nodes().last().unwrap().clone();
    assert_eq!(node.kind(), "HowTo");
    insta::assert_snapshot!(SchemaGraph::new(vec![node]).to_json());
}

#[test]
fn two_how_tos_get_their_own_ids() {
    let mut post = internal_links_post_de();
    post.blocks = vec![SchemaBlock::HowTo(how_to()), SchemaBlock::HowTo(how_to())];
    let graph = page_graph(&demo_site(), &post).unwrap();
    let ids: Vec<&str> = graph
        .nodes()
        .iter()
        .filter(|node| node.kind() == "HowTo")
        .filter_map(|node| node.id().map(khalid_core::AbsoluteUrl::as_str))
        .collect();
    assert_eq!(ids.len(), 2);
    assert!(
        ids[0].ends_with("#howto-1") && ids[1].ends_with("#howto-2"),
        "{ids:?}"
    );
}

#[test]
fn money_is_written_in_the_currency_s_own_decimals() {
    assert_eq!(Money::new(1250, "GBP").unwrap().to_string(), "12.50");
    assert_eq!(Money::new(5, "EUR").unwrap().to_string(), "0.05");
    assert_eq!(Money::new(-300, "USD").unwrap().to_string(), "-3.00");
    assert_eq!(Money::new(1500, "JPY").unwrap().to_string(), "1500");
    assert!(matches!(
        Money::new(100, "pounds"),
        Err(SeoError::Currency { .. })
    ));
    assert!(matches!(
        Money::new(100, "gbp"),
        Err(SeoError::Currency { .. })
    ));
}
