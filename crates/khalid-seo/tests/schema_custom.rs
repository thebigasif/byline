//! The custom JSON-LD field per post and per site (task 1.14, KH-SEO-3).

#![cfg(test)]

mod common;

use common::{demo_site, internal_links_post, internal_links_post_de};
use khalid_seo::{CustomSchema, SchemaGraph, SchemaNode, SchemaValue, SeoError, page_graph};

const EVENT: &str = r#"{
  "@context": "https://schema.org",
  "@type": "Event",
  "name": "Internal links clinic",
  "startDate": "2026-11-12T18:00:00+00:00",
  "eventAttendanceMode": "https://schema.org/OnlineEventAttendanceMode",
  "isAccessibleForFree": true,
  "maximumAttendeeCapacity": 40,
  "location": {
    "@type": "VirtualLocation",
    "url": "https://www.example.com/blog/clinic/"
  },
  "organizer": { "@id": "https://www.example.com/blog/#organization" }
}"#;

#[test]
fn a_post_s_custom_json_ld_joins_its_graph() {
    let mut post = internal_links_post();
    post.custom_schema = CustomSchema::parse(EVENT).unwrap();
    let graph = page_graph(&demo_site(), &post).unwrap();
    let node = graph.nodes().last().unwrap().clone();
    assert_eq!(node.kind(), "Event");
    assert_eq!(
        node.get("maximumAttendeeCapacity"),
        Some(&SchemaValue::Integer(40))
    );
    insta::assert_snapshot!(SchemaGraph::new(vec![node]).to_json());
}

#[test]
fn a_graph_gives_one_node_each_and_the_site_s_come_before_the_post_s() {
    let mut site = demo_site();
    site.custom_schema = CustomSchema::parse(
        r#"{"@context": "https://schema.org", "@graph": [
            {"@type": "Course", "name": "Links for beginners"},
            {"@type": "Book", "name": "The internal links handbook"}
        ]}"#,
    )
    .unwrap();
    let mut post = internal_links_post_de();
    post.custom_schema = CustomSchema::parse(EVENT).unwrap();
    let graph = page_graph(&site, &post).unwrap();
    let kinds: Vec<&str> = graph.nodes().iter().map(SchemaNode::kind).collect();
    assert_eq!(
        kinds.get(kinds.len() - 3..),
        Some(&["Course", "Book", "Event"][..])
    );
}

#[test]
fn a_script_in_custom_text_cannot_close_the_tag() {
    let mut post = internal_links_post();
    post.custom_schema =
        CustomSchema::parse(r#"{"@type": "Course", "name": "</script><script>alert(1)"}"#).unwrap();
    let tag = page_graph(&demo_site(), &post).unwrap().to_script_tag();
    assert_eq!(tag.matches("</script>").count(), 1, "{tag}");
}

#[test]
fn an_empty_field_adds_nothing() {
    assert_eq!(CustomSchema::parse("  \n").unwrap().nodes(), &[]);
    assert_eq!(CustomSchema::default().nodes(), &[]);
}

#[test]
fn broken_json_says_where() {
    assert_eq!(
        CustomSchema::parse("{\n  \"@type\": \"Event\",\n}"),
        Err(SeoError::CustomSchemaSyntax { line: 3, column: 1 })
    );
}

#[test]
fn each_refused_shape_names_the_place_and_the_problem() {
    let cases = [
        ("[]", ""),
        (r#""Event""#, ""),
        (r#"{"name": "No type"}"#, ""),
        (r#"{"@type": ["Event", "Thing"]}"#, "@type"),
        (r#"{"@type": "event"}"#, "@type"),
        (
            r#"{"@context": "https://example.com", "@type": "Event"}"#,
            "@context",
        ),
        (r#"{"@type": "Event", "@id": "/clinic/"}"#, "@id"),
        (r#"{"@type": "Event", "@reverse": {}}"#, "@reverse"),
        (r#"{"@type": "Event", "Name": "Clinic"}"#, "Name"),
        (r#"{"@type": "Event", "name": null}"#, "name"),
        (
            r#"{"@type": "Event", "location": {"name": "Room 2"}}"#,
            "location",
        ),
        (r#"{"@graph": [{"@type": "Event"}, 3]}"#, "@graph[1]"),
        (
            r#"{"@type": "Event", "location": {"@type": "Place", "@context": "https://schema.org"}}"#,
            "location.@context",
        ),
    ];
    for (text, place) in cases {
        let result = CustomSchema::parse(text);
        let Err(SeoError::CustomSchema { at, .. }) = &result else {
            panic!("{text} gave {result:?}");
        };
        assert_eq!(at, place, "{text}");
    }
}

#[test]
fn a_field_over_64_kb_is_refused() {
    let long = format!(
        r#"{{"@type": "Course", "name": "{}"}}"#,
        "a".repeat(64 * 1024)
    );
    assert!(matches!(
        CustomSchema::parse(&long),
        Err(SeoError::CustomSchema { .. })
    ));
}
