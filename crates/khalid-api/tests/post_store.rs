//! Posts in every status, as the Admin API reads them, and the shape it
//! sends them in.

use chrono::{TimeZone, Utc};
use khalid_api::post_store::{MemoryPostStore, PostStore};
use khalid_api::resources::{AdminPost, PostStatus};
use khalid_core::Slug;
use serde_json::json;

#[cfg(test)]
fn draft() -> AdminPost {
    AdminPost::new(
        "anchor-text-draft",
        Slug::try_new("choosing-anchor-text").unwrap(),
        "Choosing anchor text",
        "Anchor text tells readers where a link goes.",
        Utc.with_ymd_and_hms(2026, 10, 20, 8, 30, 0).unwrap(),
    )
}

#[test]
fn a_new_admin_post_is_a_draft_in_api_shape() {
    let post = draft();
    assert_eq!(post.status(), PostStatus::Draft);
    assert_eq!(post.id(), "anchor-text-draft");
    assert_eq!(
        serde_json::to_value(&post).unwrap(),
        json!({
            "id": "anchor-text-draft",
            "slug": "choosing-anchor-text",
            "title": "Choosing anchor text",
            "status": "draft",
            "markdown": "Anchor text tells readers where a link goes.",
            "published_at": null,
            "updated_at": "2026-10-20T08:30:00Z",
            "updated_by": null,
        })
    );
}

#[test]
fn statuses_are_written_in_lowercase() {
    let names: Vec<String> = [
        PostStatus::Draft,
        PostStatus::Published,
        PostStatus::Scheduled,
    ]
    .iter()
    .map(|status| {
        serde_json::to_value(status)
            .unwrap()
            .as_str()
            .unwrap()
            .to_owned()
    })
    .collect();
    assert_eq!(names, ["draft", "published", "scheduled"]);
}

#[tokio::test]
async fn the_store_finds_a_post_by_identity_or_nothing() {
    let store = MemoryPostStore::new(vec![draft()]);
    assert_eq!(store.get("anchor-text-draft").await.unwrap(), Some(draft()));
    assert_eq!(store.get("no-such-post").await.unwrap(), None);
}
