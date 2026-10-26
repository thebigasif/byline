//! The JSON-LD graph model (task 1.14).

#![cfg(test)]

use std::time::Duration;

use chrono::{NaiveDate, TimeZone, Utc};
use khalid_core::AbsoluteUrl;
use khalid_seo::{SchemaGraph, SchemaNode, SchemaValue};

fn url(text: &str) -> AbsoluteUrl {
    AbsoluteUrl::parse(text).unwrap()
}

fn demo_graph() -> SchemaGraph {
    let organization = SchemaNode::new("Organization")
        .with_id(url("https://www.example.com/blog/#organization"))
        .with("name", SchemaValue::text("Field Notes on SEO"))
        .with(
            "sameAs",
            SchemaValue::List(vec![SchemaValue::Url(url(
                "https://bsky.app/profile/fieldnotes.example.com",
            ))]),
        )
        .with(
            "foundingDate",
            SchemaValue::Date(NaiveDate::from_ymd_opt(2019, 3, 1).unwrap()),
        );
    let video = SchemaNode::new("VideoObject")
        .with("name", SchemaValue::text("Internal links in five minutes"))
        .with(
            "duration",
            SchemaValue::Duration(Duration::from_secs(3 * 3600 + 312)),
        )
        .with(
            "uploadDate",
            SchemaValue::DateTime(Utc.with_ymd_and_hms(2026, 9, 12, 10, 0, 0).unwrap()),
        )
        .with(
            "publisher",
            SchemaValue::Reference(url("https://www.example.com/blog/#organization")),
        );
    let rating = SchemaNode::new("AggregateRating")
        .with("ratingValue", SchemaValue::Number(4.5))
        .with("ratingCount", SchemaValue::Integer(128));
    let product = SchemaNode::new("Product")
        .with("name", SchemaValue::text("Link audit checklist"))
        .with("aggregateRating", SchemaValue::Node(Box::new(rating)));
    SchemaGraph::new(vec![organization, video, product])
}

#[test]
fn the_graph_prints_as_json_ld_with_its_context() {
    insta::assert_snapshot!(demo_graph().to_json());
}

#[test]
fn the_graph_is_valid_json() {
    let parsed: serde_json::Value = serde_json::from_str(&demo_graph().to_json()).unwrap();
    assert_eq!(parsed["@context"], "https://schema.org");
    assert_eq!(parsed["@graph"].as_array().unwrap().len(), 3);
    assert_eq!(parsed["@graph"][1]["duration"], "PT3H5M12S");
}

#[test]
fn the_script_tag_cannot_be_closed_by_text_in_the_graph() {
    let graph = SchemaGraph::new(vec![
        SchemaNode::new("Article").with("headline", SchemaValue::text("Stop </script><b>here</b>")),
    ]);
    let tag = graph.to_script_tag();
    assert!(
        tag.starts_with("<script type=\"application/ld+json\">"),
        "{tag}"
    );
    assert_eq!(tag.matches("</script>").count(), 1, "{tag}");
    let json = tag
        .trim_start_matches("<script type=\"application/ld+json\">")
        .trim_end_matches("</script>\n");
    let parsed: serde_json::Value = serde_json::from_str(json).unwrap();
    assert_eq!(parsed["@graph"][0]["headline"], "Stop </script><b>here</b>");
}

#[test]
fn setting_a_property_twice_keeps_its_place_and_the_last_value() {
    let mut node = SchemaNode::new("Article")
        .with("headline", SchemaValue::text("Draft title"))
        .with("inLanguage", SchemaValue::text("en"));
    node.set("headline", SchemaValue::text("Why internal links matter"));
    let names: Vec<&str> = node
        .properties()
        .iter()
        .map(|(name, _)| name.as_str())
        .collect();
    assert_eq!(names, ["headline", "inLanguage"]);
    assert_eq!(
        node.get("headline"),
        Some(&SchemaValue::text("Why internal links matter"))
    );
    assert_eq!(node.kind(), "Article");
}

#[test]
fn an_empty_graph_is_still_valid_json_ld() {
    let parsed: serde_json::Value =
        serde_json::from_str(&SchemaGraph::default().to_json()).unwrap();
    assert_eq!(parsed["@graph"], serde_json::json!([]));
}
