//! Schema templates per category or collection (task 1.14, KH-SEO-3).

#![cfg(test)]

mod common;

use common::{demo_site, internal_links_post, internal_links_post_de};
use khalid_seo::{
    MetaTemplate, PageKind, SchemaGraph, SchemaTemplate, SchemaType, SchemaValue, SeoError,
    TemplateScope, page_graph,
};

fn kind(name: &str) -> SchemaType {
    SchemaType::try_new(name).unwrap()
}

fn course_template() -> SchemaTemplate {
    let mut template = SchemaTemplate::new(
        TemplateScope::Category(String::from("On-page SEO")),
        kind("Course"),
    );
    for (property, text) in [
        ("name", "{title}"),
        ("description", "{excerpt}"),
        ("provider", "{site}"),
    ] {
        template
            .set(property, MetaTemplate::parse(text).unwrap())
            .unwrap();
    }
    template
}

#[test]
fn a_post_in_a_templated_category_carries_the_template_node() {
    let mut site = demo_site();
    site.schema_templates = vec![course_template()];
    let graph = page_graph(&site, &internal_links_post()).unwrap();
    let node = graph.nodes().last().unwrap().clone();
    assert_eq!(node.kind(), "Course");
    insta::assert_snapshot!(SchemaGraph::new(vec![node]).to_json());
}

#[test]
fn a_collection_template_matches_the_post_s_collection() {
    let mut site = demo_site();
    let mut template = SchemaTemplate::new(
        TemplateScope::Collection(String::from("guides")),
        kind("TechArticle"),
    );
    template
        .set("about", MetaTemplate::parse("{title}").unwrap())
        .unwrap();
    site.schema_templates = vec![template];
    let mut guide = internal_links_post_de();
    guide.collection = Some(String::from("guides"));
    let graph = page_graph(&site, &guide).unwrap();
    let node = graph.nodes().last().unwrap();
    assert_eq!(node.kind(), "TechArticle");
    assert_eq!(
        node.get("about"),
        Some(&SchemaValue::text("Warum interne Links wichtig sind"))
    );
}

#[test]
fn a_page_outside_the_scope_gets_no_template_node() {
    let mut site = demo_site();
    site.schema_templates = vec![course_template()];
    let graph = page_graph(&site, &internal_links_post_de()).unwrap();
    assert!(graph.nodes().iter().all(|node| node.kind() != "Course"));
}

#[test]
fn an_archive_of_the_category_gets_no_template_node() {
    let mut site = demo_site();
    site.schema_templates = vec![course_template()];
    let mut archive = internal_links_post();
    archive.kind = PageKind::Category;
    let graph = page_graph(&site, &archive).unwrap();
    assert!(graph.nodes().iter().all(|node| node.kind() != "Course"));
}

#[test]
fn a_property_whose_template_renders_empty_is_left_out() {
    let mut site = demo_site();
    let mut template = course_template();
    template
        .set("description", MetaTemplate::parse("{tagline}").unwrap())
        .unwrap();
    site.tagline = String::new();
    site.schema_templates = vec![template];
    let graph = page_graph(&site, &internal_links_post()).unwrap();
    assert_eq!(graph.nodes().last().unwrap().get("description"), None);
}

#[test]
fn a_second_template_of_the_same_type_is_skipped() {
    let mut site = demo_site();
    let mut second = course_template();
    second
        .set("name", MetaTemplate::parse("{site}").unwrap())
        .unwrap();
    site.schema_templates = vec![course_template(), second];
    let graph = page_graph(&site, &internal_links_post()).unwrap();
    let courses: Vec<_> = graph
        .nodes()
        .iter()
        .filter(|node| node.kind() == "Course")
        .collect();
    assert_eq!(courses.len(), 1);
    assert_eq!(
        courses.first().unwrap().get("name"),
        Some(&SchemaValue::text("Why internal links matter"))
    );
}

#[test]
fn a_web_page_template_does_not_take_the_page_node_s_id() {
    let mut site = demo_site();
    site.schema_templates = vec![SchemaTemplate::new(
        TemplateScope::Category(String::from("On-page SEO")),
        kind("WebPage"),
    )];
    let graph = page_graph(&site, &internal_links_post()).unwrap();
    let ids: Vec<_> = graph.nodes().iter().filter_map(|node| node.id()).collect();
    let mut unique = ids.clone();
    unique.sort_by_key(|id| id.as_str().to_owned());
    unique.dedup();
    assert_eq!(ids.len(), unique.len(), "{ids:?}");
}

#[test]
fn a_type_name_is_one_capitalised_word() {
    for bad in ["", "course", "Course Page", "Course<script>", "Café"] {
        assert!(
            matches!(SchemaType::try_new(bad), Err(SeoError::SchemaType { .. })),
            "{bad:?}"
        );
    }
    assert_eq!(kind("Course").as_str(), "Course");
}

#[test]
fn a_property_name_is_one_lower_camel_case_word() {
    let mut template = course_template();
    for bad in ["", "Name", "@id", "date published", "name\""] {
        let result = template.set(bad, MetaTemplate::parse("{title}").unwrap());
        assert!(
            matches!(result, Err(SeoError::SchemaProperty { .. })),
            "{bad:?}"
        );
    }
}
