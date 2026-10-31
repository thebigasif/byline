//! The product block: `Product` with its price and ratings (brief G15,
//! KH-SEO-3).

#![cfg(test)]

mod common;

use common::{demo_site, internal_links_post, internal_links_post_de, url};
use khalid_seo::{
    AggregateRating, Availability, IssueLevel, Money, Offer, Product, ProductReview, Rating,
    SchemaBlock, SchemaGraph, check_graph, page_graph,
};
use serde_json::{Value, json};

fn audit() -> Product {
    let mut product = Product::new("Internal link audit");
    product.description =
        String::from("A written audit of every internal link on a site of up to 500 pages.");
    product.images = vec![url("https://www.example.com/blog/media/audit.jpg")];
    let mut offer = Offer::new(Money::parse("49.00", "GBP").unwrap());
    offer.url = Some(url("https://www.example.com/blog/audit/"));
    product.offer = Some(offer);
    product.review = Some(ProductReview::new(Rating::new(4.5, 5.0).unwrap(), "Asif"));
    product.aggregate = Some(AggregateRating::new(Rating::new(4.7, 5.0).unwrap(), 38));
    product
}

/// The last node of the post's graph with `blocks`, as JSON.
fn last_node(blocks: Vec<SchemaBlock>) -> (SchemaGraph, Value) {
    let mut post = internal_links_post();
    post.blocks = blocks;
    let graph = page_graph(&demo_site(), &post).unwrap();
    let last = graph.nodes().last().unwrap().clone();
    let mut json: Value = serde_json::from_str(&SchemaGraph::new(vec![last]).to_json()).unwrap();
    (graph, json["@graph"][0].take())
}

fn blocking(graph: &SchemaGraph) -> Vec<String> {
    let issues = check_graph(graph).into_iter();
    let blocks = issues.filter(|issue| issue.level() == IssueLevel::Blocks);
    blocks
        .map(|issue| format!("{} {}", issue.kind(), issue.property()))
        .collect()
}

#[test]
fn a_product_block_writes_its_price_review_and_average() {
    let (graph, node) = last_node(vec![SchemaBlock::Product(audit())]);
    let page = "https://www.example.com/blog/why-internal-links-matter/";
    let want = json!({
        "@type": "Product",
        "@id": format!("{page}#product-1"),
        "name": "Internal link audit",
        "description": "A written audit of every internal link on a site of up to 500 pages.",
        "image": ["https://www.example.com/blog/media/audit.jpg"],
        "offers": {
            "@type": "Offer",
            "price": "49.00",
            "priceCurrency": "GBP",
            "availability": "https://schema.org/InStock",
            "url": "https://www.example.com/blog/audit/"
        },
        "review": {
            "@type": "Review",
            "reviewRating": {"@type": "Rating", "ratingValue": 4.5, "bestRating": 5.0, "worstRating": 0},
            "author": {"@type": "Person", "name": "Asif"}
        },
        "aggregateRating": {
            "@type": "AggregateRating",
            "ratingValue": 4.7, "bestRating": 5.0, "worstRating": 0, "ratingCount": 38
        }
    });
    assert_eq!(node, want);
    assert_eq!(blocking(&graph), Vec::<String>::new());
}

#[test]
fn a_product_with_only_a_price_or_only_a_review_still_passes() {
    let mut priced = audit();
    priced.review = None;
    priced.aggregate = None;
    priced.offer.as_mut().unwrap().availability = Availability::OutOfStock;
    let mut reviewed = audit();
    reviewed.offer = None;
    reviewed.aggregate = None;
    for product in [priced, reviewed] {
        let (graph, _) = last_node(vec![SchemaBlock::Product(product)]);
        assert_eq!(blocking(&graph), Vec::<String>::new());
    }
    let (_, node) = last_node(vec![SchemaBlock::Product({
        let mut gone = audit();
        gone.offer.as_mut().unwrap().availability = Availability::OutOfStock;
        gone
    })]);
    assert_eq!(
        node["offers"]["availability"],
        "https://schema.org/OutOfStock"
    );
}

#[test]
fn a_product_with_no_price_and_no_rating_blocks() {
    let (graph, _) = last_node(vec![SchemaBlock::Product(Product::new("Audit"))]);
    assert_eq!(
        blocking(&graph),
        vec![String::from("Product review, aggregateRating or offers")]
    );
}

#[test]
fn two_products_on_a_german_page_get_their_own_ids() {
    let mut post = internal_links_post_de();
    post.blocks = vec![SchemaBlock::Product(audit()), SchemaBlock::Product(audit())];
    let graph = page_graph(&demo_site(), &post).unwrap();
    let ids: Vec<&str> = graph
        .nodes()
        .iter()
        .filter(|node| node.kind() == "Product")
        .filter_map(|node| node.id()?.as_str().rsplit_once('#').map(|(_, id)| id))
        .collect();
    assert_eq!(ids, ["product-1", "product-2"]);
}
