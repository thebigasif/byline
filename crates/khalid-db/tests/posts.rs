//! Posts in the index: written by a rebuild, listed newest first, and listed
//! through an index rather than a scan (RULES.md BL-DB-1, performance.md).

use std::path::PathBuf;

use khalid_core::{Locale, Slug};
use khalid_db::{ContentIndex, Database, DbError, IndexedPost, SiteId};
use sqlx::ConnectOptions;
use sqlx::sqlite::SqliteConnectOptions;

/// A migrated database in a fresh folder, with the site `main`.
#[cfg(test)]
async fn migrated() -> (tempfile::TempDir, PathBuf, Database, SiteId) {
    let dir = tempfile::tempdir().unwrap();
    let mut path = PathBuf::from(dir.path());
    path.push("site.db");
    let db = Database::open(&path).await.unwrap();
    db.migrate().await.unwrap();
    let site = db.site(&Slug::try_new("main").unwrap()).await.unwrap();
    (dir, path, db, site)
}

#[cfg(test)]
fn locale(tag: &str) -> Locale {
    Locale::try_new(tag).unwrap()
}

#[cfg(test)]
fn post(language: &str, slug: &str, date: Option<&str>) -> IndexedPost {
    let path = format!("content/{language}/posts/{slug}.md");
    let mut post = IndexedPost::new(
        &path,
        locale(language),
        "posts",
        Slug::try_new(slug).unwrap(),
        [7; 32],
    );
    post.title = Some(format!("A post called {slug}"));
    post.description = Some("Why links between posts matter.".to_owned());
    post.date = date.map(|date| date.parse().unwrap());
    post.author = Some(Slug::try_new("asif").unwrap());
    post.category = Some(Slug::try_new("guides").unwrap());
    post.translation = Some(slug.to_owned());
    post
}

#[cfg(test)]
fn index() -> ContentIndex {
    let mut index = ContentIndex::default();
    index.posts = vec![
        post("en", "anchor-text", None),
        post("en", "internal-links", Some("2026-09-14")),
        post("de", "interne-links", Some("2026-09-15")),
        post("en", "topical-authority", Some("2026-10-01")),
    ];
    index
}

#[tokio::test]
async fn posts_come_back_as_written_newest_first_and_undated_last() {
    let (_dir, _path, db, site) = migrated().await;
    let index = index();
    let summary = db.rebuild(site, &index).await.unwrap();
    assert_eq!(summary.posts(), 4);

    let english = db.posts(site, &locale("en"), "posts", 10).await.unwrap();
    let expected = [&index.posts[3], &index.posts[1], &index.posts[0]];
    assert_eq!(english.iter().collect::<Vec<_>>(), expected);

    let german = db.posts(site, &locale("de"), "posts", 10).await.unwrap();
    assert_eq!(german, [index.posts[2].clone()]);
    let guides = db.posts(site, &locale("en"), "guides", 10).await.unwrap();
    assert_eq!(guides, []);
    let newest = db.posts(site, &locale("en"), "posts", 1).await.unwrap();
    assert_eq!(newest, [index.posts[3].clone()]);
    db.close().await;
}

#[tokio::test]
async fn a_rebuild_that_fails_keeps_the_old_index() {
    let (_dir, _path, db, site) = migrated().await;
    db.rebuild(site, &index()).await.unwrap();
    let before = db.posts(site, &locale("en"), "posts", 10).await.unwrap();
    // Two English files that both say they are the English version of one
    // piece of content cannot both be indexed.
    let mut broken = index();
    broken.posts[0].translation = Some("internal-links".to_owned());
    broken
        .posts
        .push(post("en", "a-new-post", Some("2026-10-20")));
    assert!(db.rebuild(site, &broken).await.is_err());
    assert_eq!(
        db.posts(site, &locale("en"), "posts", 10).await.unwrap(),
        before
    );
    db.close().await;
}

#[tokio::test]
async fn a_slug_changed_outside_khalid_is_reported() {
    let (_dir, path, db, site) = migrated().await;
    db.rebuild(site, &index()).await.unwrap();
    let mut connection = SqliteConnectOptions::new()
        .filename(&path)
        .connect()
        .await
        .unwrap();
    sqlx::query("UPDATE posts SET slug = 'Not A Slug' WHERE slug = 'anchor-text'")
        .execute(&mut connection)
        .await
        .unwrap();
    let error = db
        .posts(site, &locale("en"), "posts", 10)
        .await
        .unwrap_err();
    assert!(
        matches!(error, DbError::Unreadable { column: "slug", .. }),
        "{error:?}"
    );
    db.close().await;
}

#[tokio::test]
async fn listing_posts_reads_an_index_and_never_sorts() {
    let (_dir, path, db, _site) = migrated().await;
    db.close().await;
    let mut connection = SqliteConnectOptions::new()
        .filename(&path)
        .connect()
        .await
        .unwrap();
    // The same file the query macro compiles, so the plan is the real one.
    // Each row is (id, parent, notused, detail).
    let plan: Vec<(i64, i64, i64, String)> = sqlx::query_as(concat!(
        "EXPLAIN QUERY PLAN ",
        include_str!("../queries/posts_newest_first.sql")
    ))
    .fetch_all(&mut connection)
    .await
    .unwrap();
    let steps: Vec<&str> = plan.iter().map(|row| row.3.as_str()).collect();
    for step in &steps {
        assert!(step.starts_with("SEARCH"), "a step scans: {steps:?}");
        assert!(!step.contains("TEMP B-TREE"), "a step sorts: {steps:?}");
    }
    assert!(
        steps.iter().any(|step| step.contains("posts_by_date")),
        "{steps:?}"
    );
}
