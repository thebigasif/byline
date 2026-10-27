//! `reviewedBy` on a post's page node: the people who checked it, from
//! `reviewed_by` in its front matter (task 1.23, plan section 4.12).

#![cfg(test)]

mod common;

use common::{demo_site, internal_links_post, internal_links_post_de, url};
use khalid_seo::{AuthorProfile, SchemaGraph, SchemaNode, SchemaValue, SeoPage, page_graph};

fn reviewer(name: &str, slug: &str) -> AuthorProfile {
    let mut profile = AuthorProfile::new(name);
    profile.url = Some(url(&format!(
        "https://www.example.com/blog/authors/{slug}/"
    )));
    profile
}

fn reference(slug: &str) -> SchemaValue {
    SchemaValue::Reference(url(&format!(
        "https://www.example.com/blog/authors/{slug}/#person"
    )))
}

fn page_node(graph: &SchemaGraph) -> &SchemaNode {
    graph
        .nodes()
        .iter()
        .find(|node| node.kind() == "WebPage")
        .unwrap()
}

fn people(graph: &SchemaGraph) -> Vec<&SchemaNode> {
    graph
        .nodes()
        .iter()
        .filter(|node| node.kind() == "Person")
        .collect()
}

fn reviewed_post() -> SeoPage {
    let mut post = internal_links_post();
    let mut mara = reviewer("Mara Lind", "mara-lind");
    mara.job_title = Some(String::from("Search quality reviewer"));
    post.reviewers = vec![mara];
    post
}

#[test]
fn a_reviewed_post_names_its_reviewer_on_the_page_node() {
    let graph = page_graph(&demo_site(), &reviewed_post()).unwrap();
    assert_eq!(
        page_node(&graph).get("reviewedBy"),
        Some(&reference("mara-lind"))
    );
    let nodes = graph.nodes();
    let page_and_reviewer = SchemaGraph::new(vec![
        page_node(&graph).clone(),
        nodes.last().unwrap().clone(),
    ]);
    insta::assert_snapshot!(page_and_reviewer.to_json());
}

#[test]
fn several_reviewers_are_a_list_in_order() {
    let mut post = reviewed_post();
    post.reviewers.push(reviewer("Lena Vogt", "lena-vogt"));
    let graph = page_graph(&demo_site(), &post).unwrap();
    assert_eq!(
        page_node(&graph).get("reviewedBy"),
        Some(&SchemaValue::List(vec![
            reference("mara-lind"),
            reference("lena-vogt")
        ]))
    );
    assert_eq!(people(&graph).len(), 3, "the author and two reviewers");
}

#[test]
fn an_author_who_also_reviewed_is_one_person_node() {
    let mut post = internal_links_post();
    post.author_profile = Some(reviewer("Asif", "asif"));
    post.reviewers = vec![reviewer("Asif", "asif")];
    let graph = page_graph(&demo_site(), &post).unwrap();
    assert_eq!(people(&graph).len(), 1);
    assert_eq!(
        page_node(&graph).get("reviewedBy"),
        Some(&reference("asif"))
    );
}

#[test]
fn a_reviewer_without_a_page_gets_an_id_of_their_own() {
    let mut post = internal_links_post();
    post.reviewers = vec![AuthorProfile::new("Mara Lind")];
    let graph = page_graph(&demo_site(), &post).unwrap();
    let ids: Vec<&str> = people(&graph)
        .iter()
        .filter_map(|node| node.id().map(khalid_core::AbsoluteUrl::as_str))
        .collect();
    assert_eq!(
        ids,
        [
            "https://www.example.com/blog/why-internal-links-matter/#author",
            "https://www.example.com/blog/why-internal-links-matter/#reviewer-1"
        ]
    );
}

#[test]
fn a_translated_post_names_its_reviewer_too() {
    let mut post = internal_links_post_de();
    let mut mara = AuthorProfile::new("Mara Lind");
    mara.url = Some(url("https://www.example.com/blog/de/authors/mara-lind/"));
    post.reviewers = vec![mara];
    let graph = page_graph(&demo_site(), &post).unwrap();
    assert_eq!(
        page_node(&graph).get("reviewedBy"),
        Some(&SchemaValue::Reference(url(
            "https://www.example.com/blog/de/authors/mara-lind/#person"
        )))
    );
}

#[test]
fn a_post_nobody_reviewed_has_no_reviewed_by() {
    let graph = page_graph(&demo_site(), &internal_links_post()).unwrap();
    assert_eq!(page_node(&graph).get("reviewedBy"), None);
}
