//! A post's author and the entities it is about (task 1.14, KH-SEO-3).

#![cfg(test)]

mod common;

use common::{demo_site, internal_links_post, internal_links_post_de, url};
use khalid_seo::{
    AuthorProfile, Entity, EntityRole, SchemaGraph, SchemaNode, SchemaValue, SeoPage, page_graph,
};

fn kinds(graph: &SchemaGraph) -> Vec<&str> {
    graph.nodes().iter().map(SchemaNode::kind).collect()
}

fn full_post() -> SeoPage {
    let mut post = internal_links_post();
    post.word_count = Some(1240);
    post.images = vec![url(
        "https://www.example.com/blog/media/internal-links-map.png",
    )];
    let mut author = AuthorProfile::new("Asif");
    author.url = Some(url("https://www.example.com/blog/author/asif/"));
    author.job_title = Some(String::from("Founder, SerpDad"));
    author.bio = String::from("Writes about search for small teams.");
    author.same_as = vec![url("https://www.linkedin.com/in/asif-example")];
    author.knows_about = vec![
        String::from("Technical SEO"),
        String::from("Internal linking"),
    ];
    post.author_profile = Some(author);
    post.entities = vec![
        Entity::new(
            "Internal link",
            EntityRole::About,
            vec![url("https://en.wikipedia.org/wiki/Internal_link")],
        ),
        Entity::new("Googlebot", EntityRole::Mention, vec![]),
    ];
    post
}

#[test]
fn a_post_is_a_blog_posting_with_its_author() {
    let graph = page_graph(&demo_site(), &full_post()).unwrap();
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
    let article_and_author = SchemaGraph::new(graph.nodes()[4..].to_vec());
    insta::assert_snapshot!(article_and_author.to_json());
}

#[test]
fn a_post_with_a_name_only_gets_a_person_with_a_name_only() {
    let graph = page_graph(&demo_site(), &internal_links_post_de()).unwrap();
    let person = &graph.nodes()[5];
    assert_eq!(person.kind(), "Person");
    assert_eq!(person.get("name"), Some(&SchemaValue::text("Asif")));
    assert_eq!(
        person.id().map(khalid_core::AbsoluteUrl::as_str),
        Some("https://www.example.com/blog/de/warum-interne-links-wichtig-sind/#author")
    );
}

#[test]
fn a_post_without_an_author_has_no_person_node() {
    let mut post = internal_links_post();
    post.author = None;
    let graph = page_graph(&demo_site(), &post).unwrap();
    assert_eq!(kinds(&graph).last(), Some(&"BlogPosting"));
}

#[test]
fn a_post_with_no_entities_has_no_about_or_mentions() {
    let graph = page_graph(&demo_site(), &internal_links_post()).unwrap();
    assert_eq!(graph.nodes()[4].get("about"), None);
    assert_eq!(graph.nodes()[4].get("mentions"), None);
}
