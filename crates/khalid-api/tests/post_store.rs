//! Posts in every status, as the Admin API reads them, and the shape it
//! sends them in.

use chrono::{TimeZone, Utc};
use khalid_api::post_store::{MemoryPostStore, PostStore};
use khalid_api::query::{ListQuery, RawListQuery};
use khalid_api::resources::{AdminPost, LintCounts, PostSettings, PostStatus, ReviewState};
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
            "author": null,
            "status": "draft",
            "markdown": "Anchor text tells readers where a link goes.",
            "published_at": null,
            "unpublish_at": null,
            "updated_at": "2026-10-20T08:30:00Z",
            "updated_by": null,
            "settings": {
                "date": null,
                "co_authors": [],
                "excerpt": null,
                "feature_image": null,
                "featured": false,
                "review_state": "draft",
                "canonical": null,
                "robots": null,
                "share_cards": {"x": null, "facebook": null},
                "aliases": [],
                "access": "public",
                "password_protected": false,
                "template": null,
                "head_code": null,
            },
        })
    );
}

#[tokio::test]
async fn a_post_keeps_its_settings_in_the_store() {
    let mut settings = PostSettings::default();
    settings.featured = true;
    settings.review_state = ReviewState::Ready;
    let store = MemoryPostStore::new(vec![draft().with_settings(settings.clone())]);
    let found = store.get("anchor-text-draft").await.unwrap().unwrap();
    assert_eq!(found.settings(), &settings);
    let json = serde_json::to_value(&found).unwrap();
    assert_eq!(json["settings"]["review_state"], "ready");
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

#[tokio::test]
async fn the_store_lists_rows_with_their_lint_counts() {
    let store =
        MemoryPostStore::new(vec![draft()]).with_lint("anchor-text-draft", LintCounts::new(1, 4));
    let mut raw = RawListQuery::default();
    raw.filter = Some("gate:refuses".to_owned());
    let listed = store
        .list(&ListQuery::try_from(&raw).unwrap())
        .await
        .unwrap();
    assert_eq!(listed.total(), 1);
    assert_eq!(listed.items()[0].lint(), LintCounts::new(1, 4));
    raw.filter = Some("gate:passes".to_owned());
    let listed = store
        .list(&ListQuery::try_from(&raw).unwrap())
        .await
        .unwrap();
    assert_eq!(listed.total(), 0);
}
