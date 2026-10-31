//! The publish door keeps the title and text each edit replaces as an
//! earlier version, the newest first, and only when they change (plan 4.6,
//! KH-EDT-2).

use std::sync::Arc;

use chrono::{DateTime, TimeZone, Utc};
use khalid_api::post_store::MemoryPostStore;
use khalid_api::publish_door::{Announce, PublishDoor};
use khalid_api::resources::{AdminPost, PostEdit};
use khalid_api::version_store::{MAX_VERSIONS, MemoryVersionStore, PostVersion, VersionStore};
use khalid_core::{FixedClock, Slug};
use serde_json::json;

const ID: &str = "en.choosing-anchor-text";

/// Tells nobody.
struct Quiet;

impl Announce for Quiet {
    fn published(&self, _post: &AdminPost) {}
}

#[cfg(test)]
fn saved() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 20, 8, 30, 0).unwrap()
}

/// A door over one draft, keeping versions if `versions` is given.
#[cfg(test)]
fn door(versions: Option<Arc<MemoryVersionStore>>) -> PublishDoor {
    let draft = AdminPost::new(
        ID,
        Slug::try_new("choosing-anchor-text").unwrap(),
        "Choosing anchor text",
        "Anchor text tells readers where a link goes.",
        saved(),
    )
    .with_updated_by("asif");
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap());
    let posts = MemoryPostStore::new(vec![draft]).with_clock(clock);
    let door = PublishDoor::new(Arc::new(posts), Arc::new(Quiet));
    match versions {
        Some(versions) => door.with_versions(versions),
        None => door,
    }
}

/// Edits the draft with `fields`, sent with the post's current `updated_at`.
#[cfg(test)]
async fn edit(door: &PublishDoor, fields: serde_json::Value, by: &str) -> AdminPost {
    let current = door.get(ID).await.unwrap().unwrap();
    let mut fields = fields;
    fields["updated_at"] = json!(current.updated_at());
    let edit: PostEdit = serde_json::from_value(fields).unwrap();
    door.edit(ID, edit, by).await.unwrap()
}

#[tokio::test]
async fn an_edit_keeps_the_text_it_replaced_with_who_saved_it_and_when() {
    let versions = Arc::new(MemoryVersionStore::default());
    let door = door(Some(Arc::clone(&versions)));
    let text = "Anchor text tells readers and search engines where a link goes.";
    let after = edit(&door, json!({ "markdown": text }), "maria").await;
    assert_eq!(after.markdown(), text);
    let kept = door.versions(ID).await.unwrap().unwrap();
    let first = PostVersion::new(
        (saved(), Some("asif".to_owned())),
        "Choosing anchor text",
        "Anchor text tells readers where a link goes.",
    );
    assert_eq!(kept, [first]);
    edit(
        &door,
        json!({ "title": "How to choose anchor text" }),
        "asif",
    )
    .await;
    let kept = door.versions(ID).await.unwrap().unwrap();
    assert_eq!(kept.len(), 2);
    assert_eq!(kept[0].markdown(), text, "the newest first");
    assert_eq!(kept[0].saved_by(), Some("maria"));
    assert_eq!(kept[0].saved_at(), after.updated_at());
}

#[tokio::test]
async fn an_edit_that_leaves_title_and_text_alone_keeps_nothing() {
    let versions = Arc::new(MemoryVersionStore::default());
    let door = door(Some(Arc::clone(&versions)));
    edit(&door, json!({ "status": "draft" }), "asif").await;
    assert_eq!(door.versions(ID).await.unwrap(), Some(Vec::new()));
}

#[tokio::test]
async fn a_door_without_a_store_keeps_no_versions() {
    let door = door(None);
    edit(&door, json!({ "markdown": "New words." }), "asif").await;
    assert_eq!(door.versions(ID).await.unwrap(), None);
}

#[tokio::test]
async fn only_the_newest_versions_are_kept() {
    let store = MemoryVersionStore::default();
    for n in 0..=MAX_VERSIONS {
        let version = PostVersion::new((saved(), None), "Title", format!("Text {n}."));
        store.keep(ID, version).await.unwrap();
    }
    let kept = store.list(ID).await.unwrap();
    assert_eq!(kept.len(), MAX_VERSIONS);
    assert_eq!(kept[0].markdown(), format!("Text {MAX_VERSIONS}."));
    assert_eq!(kept[MAX_VERSIONS - 1].markdown(), "Text 1.");
}
