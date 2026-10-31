//! The event block: `Event` (brief G15, KH-SEO-3).

#![cfg(test)]

mod common;

use common::{at, demo_site, internal_links_post, internal_links_post_de, url};
use khalid_seo::{
    Event, EventLocation, EventStatus, IssueLevel, Money, Offer, PostalAddress, SchemaBlock,
    SchemaGraph, check_graph, page_graph,
};
use serde_json::{Value, json};

fn clinic() -> Event {
    let venue = EventLocation::Venue {
        name: String::from("The Mill"),
        address: PostalAddress::new("2 Mill Lane", "Leeds", "LS1 4AB", "GB"),
    };
    let mut event = Event::new("Internal links clinic", at(2026, 11, 12, 18), venue);
    event.description =
        String::from("An hour of fixing internal links on your own posts, with help.");
    event.end = Some(at(2026, 11, 12, 19));
    event.images = vec![url("https://www.example.com/blog/media/clinic.jpg")];
    event.offer = Some(Offer::new(Money::parse("0", "GBP").unwrap()));
    event.performer = Some(String::from("Asif"));
    event
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

fn issues(graph: &SchemaGraph, level: IssueLevel) -> Vec<String> {
    let issues = check_graph(graph).into_iter();
    let found = issues.filter(|issue| issue.level() == level);
    found
        .map(|issue| format!("{} {}", issue.kind(), issue.property()))
        .collect()
}

#[test]
fn an_event_at_a_venue_writes_every_property_google_asks_for() {
    let (graph, node) = with_block(SchemaBlock::Event(clinic()));
    let page = "https://www.example.com/blog/why-internal-links-matter/";
    let want = json!({
        "@type": "Event",
        "@id": format!("{page}#event-1"),
        "name": "Internal links clinic",
        "startDate": "2026-11-12T18:00:00Z",
        "endDate": "2026-11-12T19:00:00Z",
        "eventAttendanceMode": "https://schema.org/OfflineEventAttendanceMode",
        "eventStatus": "https://schema.org/EventScheduled",
        "location": {
            "@type": "Place",
            "name": "The Mill",
            "address": {
                "@type": "PostalAddress",
                "streetAddress": "2 Mill Lane",
                "addressLocality": "Leeds",
                "postalCode": "LS1 4AB",
                "addressCountry": "GB"
            }
        },
        "description": "An hour of fixing internal links on your own posts, with help.",
        "image": ["https://www.example.com/blog/media/clinic.jpg"],
        "offers": {
            "@type": "Offer",
            "price": "0.00",
            "priceCurrency": "GBP",
            "availability": "https://schema.org/InStock"
        },
        "organizer": {"@id": "https://www.example.com/blog/#organization"},
        "performer": {"@type": "Person", "name": "Asif"}
    });
    assert_eq!(node, want);
    assert_eq!(issues(&graph, IssueLevel::Blocks), Vec::<String>::new());
    let warns = issues(&graph, IssueLevel::Warns);
    assert!(!warns.iter().any(|w| w.starts_with("Event")), "{warns:?}");
}

#[test]
fn an_online_event_that_moved_says_so() {
    let mut event = clinic();
    event.location = EventLocation::Online(url("https://meet.example.com/clinic"));
    event.status = EventStatus::MovedOnline;
    let mut post = internal_links_post_de();
    post.blocks = vec![SchemaBlock::Event(event)];
    let graph = page_graph(&demo_site(), &post).unwrap();
    let json: Value = serde_json::from_str(&graph.to_json()).unwrap();
    let node = &json["@graph"].as_array().unwrap().last().unwrap();
    assert_eq!(
        node["location"],
        json!({"@type": "VirtualLocation", "url": "https://meet.example.com/clinic"})
    );
    assert_eq!(
        node["eventAttendanceMode"],
        "https://schema.org/OnlineEventAttendanceMode"
    );
    assert_eq!(node["eventStatus"], "https://schema.org/EventMovedOnline");
    assert_eq!(issues(&graph, IssueLevel::Blocks), Vec::<String>::new());
}
