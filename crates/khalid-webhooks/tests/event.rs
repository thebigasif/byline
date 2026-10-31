//! The webhook event catalogue: every name from `docs/standards/api.md`, each
//! once, in the `resource.verb` shape, and the same names through serde.

use std::collections::BTreeSet;

use khalid_webhooks::Event;

/// The catalogue as `docs/standards/api.md` and plan section 5.12 list it.
const DOCUMENTED: [&str; 33] = [
    "post.published",
    "post.published.edited",
    "post.unpublished",
    "post.scheduled",
    "post.deleted",
    "page.published",
    "page.published.edited",
    "page.unpublished",
    "page.scheduled",
    "page.deleted",
    "glossary-term.published",
    "glossary-term.published.edited",
    "glossary-term.unpublished",
    "glossary-term.scheduled",
    "glossary-term.deleted",
    "case-study.published",
    "case-study.published.edited",
    "case-study.unpublished",
    "case-study.scheduled",
    "case-study.deleted",
    "changelog-entry.published",
    "changelog-entry.published.edited",
    "changelog-entry.unpublished",
    "changelog-entry.scheduled",
    "changelog-entry.deleted",
    "tag.added",
    "form.submitted",
    "subscriber.added",
    "comment.added",
    "lint.blocked",
    "audit.finished",
    "site.changed",
    "alert.raised",
];

#[test]
fn the_catalogue_is_exactly_the_documented_events() {
    let names: Vec<&str> = Event::ALL.iter().map(|event| event.as_str()).collect();
    assert_eq!(names, DOCUMENTED);
}

#[test]
fn every_name_parses_back_to_its_event() {
    for event in Event::ALL {
        assert_eq!(event.as_str().parse::<Event>(), Ok(event), "{event}");
    }
}

#[test]
fn serde_uses_the_same_names() {
    for event in Event::ALL {
        let json = serde_json::to_string(&event).unwrap();
        assert_eq!(json, format!("\"{}\"", event.as_str()));
        let back: Event = serde_json::from_str(&json).unwrap();
        assert_eq!(back, event);
    }
}

#[test]
fn names_are_unique_lowercase_resource_dot_verb() {
    let unique: BTreeSet<&str> = Event::ALL.iter().map(|event| event.as_str()).collect();
    assert_eq!(unique.len(), Event::ALL.len());
    for name in unique {
        assert!(name.contains('.'), "{name}");
        assert!(
            name.chars()
                .all(|c| c.is_ascii_lowercase() || c == '.' || c == '-'),
            "{name}"
        );
    }
}

#[test]
fn an_unknown_name_is_refused_with_the_name() {
    let error = "post.liked".parse::<Event>().unwrap_err();
    assert_eq!(error.to_string(), "post.liked is not a webhook event");
    assert!(serde_json::from_str::<Event>("\"post.liked\"").is_err());
}
