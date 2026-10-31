//! The site's tags as the Admin API keeps them: grouped from the posts
//! that carry them, renamed and removed in every one (KH-API-2).

use chrono::{DateTime, TimeZone, Utc};
use khalid_api::entry_store::EntryError;
use khalid_api::resources::TagEdit;
use khalid_api::tag_store::{MemoryTags, TagStore, TaggedPost, retagged, tags_of};
use khalid_core::{FixedClock, Locale, SlugMode};
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

#[cfg(test)]
fn store() -> MemoryTags {
    MemoryTags::new(posts()).with_clock(FixedClock::new(at(27)))
}

#[cfg(test)]
fn rename(name: &str, day: u32) -> TagEdit {
    let seen = at(day).to_rfc3339();
    serde_json::from_value(json!({"name": name, "updated_at": seen})).unwrap()
}

#[tokio::test]
async fn a_rename_on_the_latest_copy_changes_every_post_that_has_the_tag() {
    let tags = store();
    let renamed = tags
        .rename("en.internal-links", rename("internal linking", 20))
        .await;
    let renamed = serde_json::to_value(renamed.unwrap()).unwrap();
    assert_eq!(renamed["id"], "en.internal-linking");
    assert_eq!(renamed["posts"], 2);
    assert_eq!(renamed["updated_at"], "2026-10-27T09:00:00Z");
    let ids: Vec<_> = tags
        .list()
        .await
        .unwrap()
        .iter()
        .map(|t| t.id().to_owned())
        .collect();
    assert_eq!(
        ids,
        [
            "de.interne-links",
            "en.anchor-text",
            "en.internal-linking",
            "en.titles"
        ]
    );
}

#[tokio::test]
async fn a_stale_missing_or_nameless_rename_changes_nothing() {
    let tags = store();
    let stale = tags.rename("en.internal-links", rename("links", 18)).await;
    assert!(matches!(stale, Err(EntryError::Stale { updated_at }) if updated_at == at(20)));
    let missing = tags.rename("en.nothing", rename("links", 20)).await;
    assert!(matches!(missing, Err(EntryError::NotFound)));
    let nameless = tags.rename("en.internal-links", rename("!!", 20)).await;
    assert!(matches!(
        nameless,
        Err(EntryError::Unsupported { field: "name", .. })
    ));
    assert_eq!(
        tags.list().await.unwrap(),
        tags_of(&posts(), SlugMode::Ascii)
    );
}

#[tokio::test]
async fn a_tag_is_removed_from_every_post_on_its_latest_copy() {
    let tags = store();
    let stale = tags.remove("en.internal-links", at(18)).await;
    assert!(matches!(stale, Err(EntryError::Stale { .. })));
    tags.remove("en.internal-links", at(20)).await.unwrap();
    let left = tags.list().await.unwrap();
    assert!(
        left.iter().all(|tag| tag.id() != "en.internal-links"),
        "{left:?}"
    );
    assert_eq!(left.len(), 3);
}
