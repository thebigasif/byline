//! An update after a save: only the posts it names are written again or
//! removed, and the index ends the same as a whole rebuild would leave it.

use std::path::PathBuf;

use khalid_core::{Locale, Slug};
use khalid_db::{ContentIndex, Database, IndexedPost, PostChanges, SiteId};

/// A migrated database in a fresh folder, with the site `main`.
#[cfg(test)]
async fn migrated() -> (tempfile::TempDir, Database, SiteId) {
    let dir = tempfile::tempdir().unwrap();
    let mut path = PathBuf::from(dir.path());
    path.push("site.db");
    let db = Database::open(&path).await.unwrap();
    db.migrate().await.unwrap();
    let site = db.site(&Slug::try_new("main").unwrap()).await.unwrap();
    (dir, db, site)
}

#[cfg(test)]
fn post(slug: &str, date: &str, title: &str) -> IndexedPost {
    let path = format!("content/en/posts/{slug}.md");
    let locale = Locale::try_new("en").unwrap();
    let mut post = IndexedPost::new(
        &path,
        locale,
        "posts",
        Slug::try_new(slug).unwrap(),
        [1; 32],
    );
    post.title = Some(title.to_owned());
    post.date = Some(date.parse().unwrap());
    post.translation = Some(slug.to_owned());
    post
}

#[cfg(test)]
fn index(posts: Vec<IndexedPost>) -> ContentIndex {
    let mut index = ContentIndex::default();
    index.posts = posts;
    index
}

#[cfg(test)]
async fn titles(db: &Database, site: SiteId) -> Vec<String> {
    let english = Locale::try_new("en").unwrap();
    let rows = db.posts(site, &english, "posts", u32::MAX).await.unwrap();
    rows.into_iter().filter_map(|row| row.title).collect()
}

#[tokio::test]
async fn an_update_writes_the_named_posts_and_leaves_the_rest() {
    let (_dir, db, site) = migrated().await;
    let before = index(vec![
        post("anchor-text", "2026-09-01", "Anchor text"),
        post("internal-links", "2026-09-14", "Internal links"),
        post("site-speed", "2026-10-01", "Site speed"),
    ]);
    db.rebuild(site, &before).await.unwrap();

    let after = index(vec![
        post("anchor-text", "2026-09-01", "Anchor text that helps"),
        post("internal-links", "2026-09-14", "Internal links"),
        post("topic-clusters", "2026-10-05", "Topic clusters"),
    ]);
    let mut changes = PostChanges::default();
    changes
        .written
        .insert("content/en/posts/anchor-text.md".to_owned());
    changes
        .written
        .insert("content/en/posts/topic-clusters.md".to_owned());
    changes
        .removed
        .insert("content/en/posts/site-speed.md".to_owned());
    db.update(site, &after, &changes).await.unwrap();

    let updated = titles(&db, site).await;
    assert_eq!(
        updated,
        ["Topic clusters", "Internal links", "Anchor text that helps"]
    );
    db.rebuild(site, &after).await.unwrap();
    assert_eq!(titles(&db, site).await, updated, "the same as a rebuild");
    db.close().await;
}

#[tokio::test]
async fn an_update_that_breaks_a_rule_leaves_the_index_as_it_was() {
    let (_dir, db, site) = migrated().await;
    let before = index(vec![
        post("anchor-text", "2026-09-01", "Anchor text"),
        post("internal-links", "2026-09-14", "Internal links"),
    ]);
    db.rebuild(site, &before).await.unwrap();

    let mut clash = post("anchor-text", "2026-09-01", "Anchor text, again");
    clash.translation = Some("internal-links".to_owned());
    let after = index(vec![
        clash,
        post("internal-links", "2026-09-14", "Internal links"),
    ]);
    let mut changes = PostChanges::default();
    changes
        .written
        .insert("content/en/posts/anchor-text.md".to_owned());
    assert!(db.update(site, &after, &changes).await.is_err());

    assert_eq!(titles(&db, site).await, ["Internal links", "Anchor text"]);
    db.close().await;
}
