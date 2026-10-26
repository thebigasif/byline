//! The Admin API's posts on a copy of the demo site.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{TimeZone, Utc};
use khalid_api::post_store::CreatePostError;
use khalid_api::resources::{PostDraft, PostStatus};
use khalid_core::Environment;

use super::*;

fn copy_dir(from: &Path, into: &Path) {
    fs::create_dir_all(into).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let mut target = into.to_path_buf();
        target.push(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// A copy of the demo site and its posts.
fn demo() -> (tempfile::TempDir, SitePosts) {
    let mut fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fixture.push("../../fixtures/demo-site");
    let temp = tempfile::tempdir().unwrap();
    copy_dir(&fixture, temp.path());
    let config = Config::load(temp.path(), &Environment::empty()).unwrap();

    let posts = SitePosts::new(temp.path(), config);
    (temp, posts)
}

fn within(dir: &Path, name: &str) -> PathBuf {
    let mut path = dir.to_path_buf();
    path.push(name);
    path
}

/// A draft that passes every blocking rule once dated.
fn good_draft() -> PostDraft {
    let markdown = "Anchor text is the visible words of a link.\n\n\
        Read [why internal links matter](/blog/why-internal-links-matter/) first.";
    PostDraft::new("Choosing anchor text", None, markdown)
        .with_author(Some(Slug::try_new("asif").unwrap()))
}

#[test]
fn a_new_post_is_a_draft_file_made_once_per_key() {
    let (site, posts) = demo();
    let made = posts
        .0
        .create_now(&good_draft(), "1111222233334444", Some("k1"))
        .unwrap();
    assert_eq!(made.id(), "en.choosing-anchor-text");
    assert_eq!(made.status(), PostStatus::Draft);
    assert_eq!(made.author().map(Slug::as_str), Some("asif"));
    assert_eq!(made.updated_by(), Some("1111222233334444"));
    let file = within(site.path(), "drafts/en/choosing-anchor-text.md");
    let text = fs::read_to_string(&file).unwrap();
    let front =
        "---\ntitle: \"Choosing anchor text\"\nslug: choosing-anchor-text\nauthor: asif\n---\n";
    assert!(text.starts_with(front), "{text}");

    let again = posts
        .0
        .create_now(&good_draft(), "1111222233334444", Some("k1"))
        .unwrap();
    assert_eq!(again.id(), made.id());
    let taken = posts
        .0
        .create_now(&good_draft(), "1111222233334444", Some("k2"));
    assert!(
        matches!(taken, Err(CreatePostError::SlugTaken(_))),
        "{taken:?}"
    );
    let demo_slug = PostDraft::new("Why internal links matter", None, "Words.");
    let taken = posts.0.create_now(&demo_slug, "1111222233334444", None);
    assert!(
        matches!(taken, Err(CreatePostError::SlugTaken(_))),
        "a published slug is taken too"
    );
}

#[test]
fn a_draft_and_a_published_post_read_back_by_identity() {
    let (_site, posts) = demo();
    posts.0.create_now(&good_draft(), "k", None).unwrap();
    let draft = posts.0.get_now("en.choosing-anchor-text").unwrap().unwrap();
    assert_eq!(draft.markdown(), good_draft().markdown());
    assert_eq!(draft.published_at(), None);

    let live = posts
        .0
        .get_now("de.warum-interne-links-wichtig-sind")
        .unwrap()
        .unwrap();
    assert_eq!(live.status(), PostStatus::Published);
    assert_eq!(
        live.published_at(),
        Some(Utc.with_ymd_and_hms(2026, 9, 14, 0, 0, 0).unwrap())
    );
    assert!(
        !live.markdown().is_empty() && !live.markdown().contains("translation:"),
        "{}",
        live.markdown()
    );

    for unknown in [
        "en.no-such-post",
        "fr.why-internal-links-matter",
        "no-language",
        "en.Bad Slug",
    ] {
        assert!(posts.0.get_now(unknown).unwrap().is_none(), "{unknown}");
    }
}
