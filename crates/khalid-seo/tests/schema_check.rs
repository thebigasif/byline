//! Google's required and recommended properties (task 1.14, KH-SEO-3).

#![cfg(test)]

use khalid_seo::{CustomSchema, IssueLevel, SchemaIssue, check_graph};

fn check(json: &str) -> Vec<SchemaIssue> {
    let nodes = CustomSchema::parse(json).unwrap().nodes().to_vec();
    check_graph(&khalid_seo::SchemaGraph::new(nodes))
}

/// The type and property of each issue that blocks.
fn blocking(issues: &[SchemaIssue]) -> Vec<(&str, &str)> {
    let blocks = issues.iter().filter(|i| i.level() == IssueLevel::Blocks);
    blocks.map(|i| (i.kind(), i.property())).collect()
}

#[test]
fn an_event_without_a_start_date_blocks_and_names_what_why_and_fix() {
    let issues = check(
        r#"{"@type": "Event", "name": "Internal links clinic",
            "location": {"@type": "Place", "address": "2 Mill Lane, Leeds"}}"#,
    );
    assert_eq!(blocking(&issues), vec![("Event", "startDate")]);
    let message = issues.first().unwrap().message();
    assert_eq!(message.what(), "This Event has no startDate.");
    assert_eq!(
        message.why(),
        "Google shows an event in its event results only when it has one."
    );
    assert_eq!(message.fix(), "Add startDate to the Event.");
}

#[test]
fn a_missing_recommended_property_warns() {
    let issues = check(
        r#"{"@type": "Recipe", "name": "Office flapjacks",
            "image": "https://www.example.com/blog/media/flapjacks.jpg"}"#,
    );
    assert_eq!(blocking(&issues), Vec::new());
    let description = issues.iter().find(|i| i.property() == "description");
    assert_eq!(
        description.unwrap().message().why(),
        "Google can still show a recipe card without it, but shows less."
    );
}

#[test]
fn one_of_several_properties_is_enough() {
    let without = check(r#"{"@type": "Product", "name": "Link audit"}"#);
    assert_eq!(
        blocking(&without),
        vec![("Product", "review, aggregateRating or offers")]
    );
    assert_eq!(
        without.first().unwrap().message().what(),
        "This Product has none of review, aggregateRating or offers."
    );
    let with = check(
        r#"{"@type": "Product", "name": "Link audit",
            "offers": {"@type": "Offer", "price": 49, "priceCurrency": "GBP"}}"#,
    );
    assert_eq!(blocking(&with), Vec::new());
}

#[test]
fn nested_nodes_are_checked_and_only_a_review_alone_needs_its_item() {
    let nested = check(
        r#"{"@type": "Product", "name": "Link audit",
            "review": {"@type": "Review", "author": {"@type": "Person", "name": "Asif"},
                       "reviewRating": {"@type": "Rating"}}}"#,
    );
    assert_eq!(blocking(&nested), vec![("Rating", "ratingValue")]);
    let alone = check(
        r#"{"@type": "Review", "author": {"@type": "Person", "name": "Asif"},
            "reviewRating": {"@type": "Rating", "ratingValue": 4}}"#,
    );
    assert_eq!(blocking(&alone), vec![("Review", "itemReviewed")]);
}

#[test]
fn an_empty_text_counts_as_missing() {
    let issues = check(r#"{"@type": "WebSite", "name": " ", "url": "https://www.example.com/"}"#);
    assert_eq!(blocking(&issues), vec![("WebSite", "name")]);
}

#[test]
fn types_google_no_longer_shows_raise_nothing() {
    for json in [
        r#"{"@type": "FAQPage"}"#,
        r#"{"@type": "HowTo"}"#,
        r#"{"@type": "Course"}"#,
        r#"{"@type": "Book", "name": "The internal links handbook"}"#,
    ] {
        assert_eq!(check(json), Vec::new(), "{json}");
    }
}

#[test]
fn every_message_is_three_plain_sentences() {
    let issues = check(
        r#"{"@graph": [{"@type": "Event"}, {"@type": "VideoObject"},
            {"@type": "SoftwareApplication"}, {"@type": "Dataset"},
            {"@type": "LocalBusiness"}, {"@type": "ProfilePage"}]}"#,
    );
    assert!(issues.len() > 20);
    for issue in &issues {
        let message = issue.message();
        for sentence in [message.what(), message.why(), message.fix()] {
            assert!(sentence.ends_with('.'), "{sentence}");
            assert!(!sentence.contains('\u{2014}'), "{sentence}");
            assert_eq!(sentence.matches(". ").count(), 0, "{sentence}");
        }
    }
}
