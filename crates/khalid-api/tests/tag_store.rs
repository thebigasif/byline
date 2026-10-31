//! The site's tags as the Admin API keeps them: grouped from the posts
//! that carry them, renamed and removed in every one (KH-API-2).

use chrono::{DateTime, TimeZone, Utc};
use khalid_api::tag_store::{TaggedPost, retagged, tags_of};
use khalid_core::{Locale, SlugMode};
use serde_json::json;

#[cfg(test)]
fn at(day: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, day, 9, 0, 0).unwrap()
}

#[cfg(test)]
fn post(language: &str, tags: &[&str], day: u32) -> TaggedPost {
    TaggedPost {
        language: Locale::try_new(language).unwrap(),
        tags: tags.iter().map(|&tag| tag.to_owned()).collect(),
        updated_at: at(day),
    }
}

/// Three English posts and a German one.
#[cfg(test)]
fn posts() -> Vec<TaggedPost> {
    vec![
        post("en", &["internal links", "anchor text"], 18),
        post("en", &["Internal Links"], 20),
        post("en", &["titles"], 25),
        post("de", &["interne Links"], 19),
    ]
}

#[test]
fn tags_are_grouped_by_language_and_slug_with_their_last_change() {
    let tags = serde_json::to_value(tags_of(&posts(), SlugMode::Ascii)).unwrap();
    assert_eq!(tags[0]["id"], "de.interne-links");
    assert_eq!(tags[1]["id"], "en.anchor-text");
    assert_eq!(
        tags[2],
        json!({
            "id": "en.internal-links", "slug": "internal-links", "language": "en",
            "name": "internal links", "posts": 2, "updated_at": "2026-10-20T09:00:00Z",
        })
    );
    assert_eq!(tags[3]["id"], "en.titles");
}

#[test]
fn a_rename_onto_a_tag_the_post_has_keeps_one_and_a_post_without_it_is_left() {
    let first = &posts()[0];
    let renamed = retagged(
        first,
        "en.internal-links",
        Some("Anchor Text"),
        SlugMode::Ascii,
    );
    assert_eq!(renamed, Some(vec!["Anchor Text".to_owned()]));
    let removed = retagged(first, "en.anchor-text", None, SlugMode::Ascii);
    assert_eq!(removed, Some(vec!["internal links".to_owned()]));
    assert_eq!(retagged(first, "en.titles", None, SlugMode::Ascii), None);
}
