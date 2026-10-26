//! Google's required and recommended properties (task 1.14, KH-SEO-3).

#![cfg(test)]

use khalid_seo::{CustomSchema, IssueLevel, SchemaGraph, SchemaIssue, check_graph};

fn graph(json: &str) -> SchemaGraph {
    SchemaGraph::new(CustomSchema::parse(json).unwrap().nodes().to_vec())
}

fn found(issues: &[SchemaIssue]) -> Vec<(IssueLevel, &str, &str)> {
    issues
        .iter()
        .map(|issue| (issue.level(), issue.kind(), issue.property()))
        .collect()
}

#[test]
fn an_event_without_a_start_date_blocks_and_names_what_why_and_fix() {
    let issues = check_graph(&graph(
        r#"{"@type": "Event", "name": "Internal links clinic",
            "location": {"@type": "Place", "address": "2 Mill Lane, Leeds"}}"#,
    ));
    let blocking: Vec<_> = issues
        .iter()
        .filter(|issue| issue.level() == IssueLevel::Blocks)
        .collect();
    assert_eq!(blocking.len(), 1, "{issues:?}");
    let message = blocking.first().unwrap().message();
    assert_eq!(message.what(), "This Event has no startDate.");
    assert_eq!(
        message.why(),
        "Google shows an event in its event results only when it has one."
    );
    assert_eq!(message.fix(), "Add startDate to the Event.");
}

#[test]
fn a_missing_recommended_property_warns() {
    let issues = check_graph(&graph(
        r#"{"@type": "Recipe", "name": "Office flapjacks",
            "image": "https://www.example.com/blog/media/flapjacks.jpg"}"#,
    ));
    assert!(
        issues
            .iter()
            .all(|issue| issue.level() == IssueLevel::Warns)
    );
    let description = issues
        .iter()
        .find(|issue| issue.property() == "description")
        .unwrap();
    assert_eq!(
        description.message().why(),
        "Google can still show a recipe card without it, but shows less."
    );
}

#[test]
fn one_of_several_properties_is_enough() {
    let without = check_graph(&graph(r#"{"@type": "Product", "name": "Link audit"}"#));
    let blocking = without
        .iter()
        .find(|issue| issue.level() == IssueLevel::Blocks)
        .unwrap();
    assert_eq!(blocking.property(), "review, aggregateRating or offers");
    assert_eq!(
        blocking.message().what(),
        "This Product has none of review, aggregateRating or offers."
    );
    let with = check_graph(&graph(
        r#"{"@type": "Product", "name": "Link audit",
            "offers": {"@type": "Offer", "price": 49, "priceCurrency": "GBP"}}"#,
    ));
    assert!(
        with.iter().all(|issue| issue.level() == IssueLevel::Warns),
        "{with:?}"
    );
}

#[test]
fn nested_nodes_are_checked_and_a_nested_review_needs_no_item_reviewed() {
    let issues = check_graph(&graph(
        r#"{"@type": "Product", "name": "Link audit",
            "review": {"@type": "Review", "author": {"@type": "Person", "name": "Asif"},
                       "reviewRating": {"@type": "Rating"}}}"#,
    ));
    assert_eq!(
        found(&issues)
            .into_iter()
            .filter(|(level, _, _)| *level == IssueLevel::Blocks)
            .collect::<Vec<_>>(),
        vec![(IssueLevel::Blocks, "Rating", "ratingValue")]
    );
}

#[test]
fn a_review_on_its_own_needs_the_item_it_reviews() {
    let issues = check_graph(&graph(
        r#"{"@type": "Review", "author": {"@type": "Person", "name": "Asif"},
            "reviewRating": {"@type": "Rating", "ratingValue": 4}}"#,
    ));
    assert!(found(&issues).contains(&(IssueLevel::Blocks, "Review", "itemReviewed")));
}

#[test]
fn an_empty_text_counts_as_missing() {
    let issues = check_graph(&graph(
        r#"{"@type": "WebSite", "name": "", "url": "https://www.example.com/blog/"}"#,
    ));
    assert_eq!(
        found(&issues),
        vec![(IssueLevel::Blocks, "WebSite", "name")]
    );
}

#[test]
fn types_google_no_longer_shows_raise_nothing() {
    for json in [
        r#"{"@type": "FAQPage"}"#,
        r#"{"@type": "HowTo"}"#,
        r#"{"@type": "Course"}"#,
        r#"{"@type": "Book", "name": "The internal links handbook"}"#,
    ] {
        assert_eq!(check_graph(&graph(json)), Vec::new(), "{json}");
    }
}

#[test]
fn every_message_is_three_plain_sentences() {
    let issues = check_graph(&graph(
        r#"{"@graph": [{"@type": "Event"}, {"@type": "VideoObject"},
            {"@type": "SoftwareApplication"}, {"@type": "Dataset"},
            {"@type": "LocalBusiness"}, {"@type": "ProfilePage"}]}"#,
    ));
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
