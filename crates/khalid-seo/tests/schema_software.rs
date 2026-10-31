//! The software block: `SoftwareApplication` and its mobile and web kinds
//! (brief G15, KH-SEO-3).

#![cfg(test)]

mod common;

use common::{demo_site, internal_links_post, internal_links_post_de};
use khalid_seo::{
    AggregateRating, AppKind, IssueLevel, Money, Offer, ProductReview, Rating, SchemaBlock,
    SchemaGraph, SoftwareApplication, check_graph, page_graph,
};
use serde_json::{Value, json};

fn khalid() -> SoftwareApplication {
    let mut app = SoftwareApplication::new("Khalid", Offer::new(Money::parse("0", "GBP").unwrap()));
    app.category = Some(String::from("BusinessApplication"));
    app.operating_system = Some(String::from("Linux, macOS, Windows"));
    app.aggregate = Some(AggregateRating::new(Rating::new(4.8, 5.0).unwrap(), 120));
    app
}

/// The graph of the English post with `block`, and its last node as JSON.
fn with_block(block: SchemaBlock) -> (SchemaGraph, Value) {
    let mut post = internal_links_post();
    post.blocks = vec![block];
    let graph = page_graph(&demo_site(), &post).unwrap();
    let last = graph.nodes().last().unwrap().clone();
    let mut json: Value = serde_json::from_str(&SchemaGraph::new(vec![last]).to_json()).unwrap();
    (graph, json["@graph"][0].take())
}

fn blocking(graph: &SchemaGraph) -> Vec<String> {
    let issues = check_graph(graph).into_iter();
    let found = issues.filter(|issue| issue.level() == IssueLevel::Blocks);
    found
        .map(|issue| format!("{} {}", issue.kind(), issue.property()))
        .collect()
}

#[test]
fn an_app_writes_its_kind_price_and_average() {
    let (graph, node) = with_block(SchemaBlock::Software(khalid()));
    let page = "https://www.example.com/blog/why-internal-links-matter/";
    let want = json!({
        "@type": "SoftwareApplication",
        "@id": format!("{page}#app-1"),
        "name": "Khalid",
        "applicationCategory": "BusinessApplication",
        "operatingSystem": "Linux, macOS, Windows",
        "offers": {
            "@type": "Offer",
            "price": "0.00",
            "priceCurrency": "GBP",
            "availability": "https://schema.org/InStock"
        },
        "aggregateRating": {
            "@type": "AggregateRating",
            "ratingValue": 4.8, "bestRating": 5.0, "worstRating": 0, "ratingCount": 120
        }
    });
    assert_eq!(node, want);
    assert_eq!(blocking(&graph), Vec::<String>::new());
}

#[test]
fn a_mobile_app_with_a_review_passes_and_one_with_no_score_blocks() {
    let mut reviewed = khalid();
    reviewed.kind = AppKind::Mobile;
    reviewed.aggregate = None;
    reviewed.review = Some(ProductReview::new(Rating::new(4.0, 5.0).unwrap(), "Asif"));
    let (graph, node) = with_block(SchemaBlock::Software(reviewed));
    assert_eq!(node["@type"], "MobileApplication");
    assert_eq!(node["review"]["author"]["name"], "Asif");
    assert_eq!(blocking(&graph), Vec::<String>::new());

    let mut unscored = khalid();
    unscored.kind = AppKind::Web;
    unscored.aggregate = None;
    let mut post = internal_links_post_de();
    post.blocks = vec![SchemaBlock::Software(unscored)];
    let graph = page_graph(&demo_site(), &post).unwrap();
    assert_eq!(
        blocking(&graph),
        vec![String::from("WebApplication aggregateRating or review")]
    );
}
