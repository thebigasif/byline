//! The whole graph of one page, and its page node (task 1.14, KH-SEO-3).

#![cfg(test)]

mod common;

use common::{demo_site, internal_links_post, internal_links_post_de, locale, path, url};
use khalid_seo::{PageKind, SchemaGraph, SchemaNode, SeoPage, ShareImage, UrlPath, page_graph};

fn kinds(graph: &SchemaGraph) -> Vec<&str> {
    graph.nodes().iter().map(SchemaNode::kind).collect()
}

#[test]
fn the_page_node_of_a_post() {
    let mut post = internal_links_post();
    post.share_image = Some(ShareImage::new(
        url("https://www.example.com/blog/media/internal-links.png"),
        1200,
        630,
        "A site map with arrows between pages",
    ));
    post.speakable = vec![String::from(".post-summary")];
    let graph = page_graph(&demo_site(), &post).unwrap();
    let page_node = graph
        .nodes()
        .iter()
        .find(|node| node.kind() == "WebPage")
        .unwrap();
    insta::assert_snapshot!(SchemaGraph::new(vec![page_node.clone()]).to_json());
}

#[test]
fn every_page_graph_holds_publisher_site_breadcrumbs_and_page() {
    let graph = page_graph(&demo_site(), &internal_links_post_de()).unwrap();
    assert_eq!(
        kinds(&graph),
        [
            "Organization",
            "WebSite",
            "BreadcrumbList",
            "WebPage",
            "BlogPosting",
            "Person"
        ]
    );
    let page = &graph.nodes()[3];
    assert_eq!(
        page.get("inLanguage"),
        Some(&khalid_seo::SchemaValue::text("de"))
    );
}

#[test]
fn each_page_kind_gets_its_schema_type() {
    let cases = [
        (PageKind::Home, "WebPage"),
        (PageKind::Page, "WebPage"),
        (PageKind::Tag, "CollectionPage"),
        (PageKind::Category, "CollectionPage"),
        (PageKind::Collection, "CollectionPage"),
        (PageKind::GlossaryIndex, "CollectionPage"),
        (PageKind::Author, "ProfilePage"),
        (PageKind::Search, "SearchResultsPage"),
    ];
    for (kind, expected) in cases {
        let page = SeoPage::new(kind, locale("en"), path("/somewhere/"), "Somewhere");
        let graph = page_graph(&demo_site(), &page).unwrap();
        assert_eq!(graph.nodes()[3].kind(), expected, "{kind:?}");
    }
}

#[test]
fn the_home_page_node_is_named_after_the_site() {
    let home = SeoPage::new(PageKind::Home, locale("en"), UrlPath::root(), "");
    let graph = page_graph(&demo_site(), &home).unwrap();
    assert_eq!(
        graph.nodes()[3].get("name"),
        Some(&khalid_seo::SchemaValue::text("Field Notes on SEO"))
    );
}
