//! The dataset block and the listicle: `Dataset` and `ItemList` (brief
//! G15, KH-SEO-3).

#![cfg(test)]

mod common;

use common::{demo_site, internal_links_post, internal_links_post_de, url};
use khalid_seo::{
    Dataset, IssueLevel, ItemList, ListEntry, SchemaBlock, SchemaGraph, SeoPage, check_graph,
    page_graph,
};
use serde_json::{Value, json};

const PAGE: &str = "https://www.example.com/blog/why-internal-links-matter/";

/// The graph of `post` with `block`, and its last node as JSON.
fn with_block(mut post: SeoPage, block: SchemaBlock) -> (SchemaGraph, Value) {
    post.blocks = vec![block];
    let graph = page_graph(&demo_site(), &post).unwrap();
    let last = graph.nodes().last().unwrap().clone();
    let mut json: Value = serde_json::from_str(&SchemaGraph::new(vec![last]).to_json()).unwrap();
    (graph, json["@graph"][0].take())
}

fn issues(graph: &SchemaGraph, kind: &str) -> Vec<String> {
    let issues = check_graph(graph).into_iter();
    let found = issues.filter(|issue| issue.kind() == kind || issue.level() == IssueLevel::Blocks);
    found
        .map(|issue| format!("{} {}", issue.kind(), issue.property()))
        .collect()
}

fn links_data() -> Dataset {
    let mut data = Dataset::new(
        "Internal links on 1,000 small blogs",
        "How many internal links each post has, counted on 1,000 small blogs in 2026.",
    );
    data.keywords = vec![String::from("internal links"), String::from("blogs")];
    data.license = Some(url("https://creativecommons.org/licenses/by/4.0/"));
    data.download = Some(url(
        "https://www.example.com/blog/data/internal-links-2026.csv",
    ));
    data
}

#[test]
fn a_dataset_writes_its_page_licence_creator_and_download() {
    let mut post = internal_links_post();
    post.author = Some(String::from("Asif"));
    let (graph, node) = with_block(post, SchemaBlock::Dataset(links_data()));
    let want = json!({
        "@type": "Dataset",
        "@id": format!("{PAGE}#dataset-1"),
        "name": "Internal links on 1,000 small blogs",
        "description": "How many internal links each post has, counted on 1,000 small blogs in 2026.",
        "url": PAGE,
        "keywords": ["internal links", "blogs"],
        "license": "https://creativecommons.org/licenses/by/4.0/",
        "creator": {"@type": "Person", "name": "Asif"},
        "distribution": {
            "@type": "DataDownload",
            "contentUrl": "https://www.example.com/blog/data/internal-links-2026.csv"
        }
    });
    assert_eq!(node, want);
    assert_eq!(issues(&graph, "Dataset"), Vec::<String>::new());
}

#[test]
fn a_bare_dataset_still_passes_and_only_warns() {
    let mut post = internal_links_post_de();
    post.author = None;
    let bare = Dataset::new("Anchor texts", "Anchor texts counted on 200 blogs.");
    let (graph, _) = with_block(post, SchemaBlock::Dataset(bare));
    assert_eq!(
        issues(&graph, "Dataset"),
        ["Dataset creator", "Dataset keywords", "Dataset license"]
    );
}

#[test]
fn a_listicle_numbers_its_entries_from_one() {
    let list = ItemList::new(
        "Two posts to read next",
        vec![
            ListEntry::new("Why internal links matter", url(PAGE)),
            ListEntry::new(
                "Anchor text",
                url("https://www.example.com/blog/anchor-text/"),
            ),
        ],
    );
    let (graph, node) = with_block(internal_links_post(), SchemaBlock::List(list));
    let want = json!({
        "@type": "ItemList",
        "@id": format!("{PAGE}#list-1"),
        "name": "Two posts to read next",
        "numberOfItems": 2,
        "itemListElement": [
            {"@type": "ListItem", "position": 1, "name": "Why internal links matter", "url": PAGE},
            {"@type": "ListItem", "position": 2, "name": "Anchor text",
             "url": "https://www.example.com/blog/anchor-text/"}
        ]
    });
    assert_eq!(node, want);
    assert_eq!(issues(&graph, "ItemList"), Vec::<String>::new());
}

#[test]
fn an_empty_listicle_blocks() {
    let empty = ItemList::new("Nothing yet", Vec::new());
    let (graph, _) = with_block(internal_links_post(), SchemaBlock::List(empty));
    assert_eq!(issues(&graph, "ItemList"), ["ItemList itemListElement"]);
}
