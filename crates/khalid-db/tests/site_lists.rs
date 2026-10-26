//! The site's lists in the index: a rebuild writes the authors, categories
//! and tags it is given, and they read back in that order, in every
//! language, until the next rebuild replaces them.

use std::path::PathBuf;

use khalid_core::{Locale, Slug};
use khalid_db::{ContentIndex, Database, IndexedAuthor, IndexedTerm};

/// A migrated database in a fresh folder.
#[cfg(test)]
async fn migrated() -> (tempfile::TempDir, Database) {
    let dir = tempfile::tempdir().unwrap();
    let mut path = PathBuf::from(dir.path());
    path.push("site.db");
    let db = Database::open(&path).await.unwrap();
    db.migrate().await.unwrap();
    (dir, db)
}

#[cfg(test)]
fn term(locale: &str, slug: &str, name: &str) -> IndexedTerm {
    IndexedTerm::new(
        Locale::try_new(locale).unwrap(),
        Slug::try_new(slug).unwrap(),
        name,
    )
}

#[cfg(test)]
fn author(locale: &str, slug: &str, name: &str) -> IndexedAuthor {
    let path = format!("content/{locale}/authors/{slug}.md");
    IndexedAuthor::new(
        &path,
        Locale::try_new(locale).unwrap(),
        Slug::try_new(slug).unwrap(),
        name,
        [7; 32],
    )
}

/// The demo site's authors and categories, and two tags.
#[cfg(test)]
fn demo() -> ContentIndex {
    let mut index = ContentIndex::default();
    index.authors = vec![
        author("en", "lena-vogt", "Lena Vogt"),
        author("en", "asif", "Asif"),
        author("de", "asif", "Asif"),
    ];
    index.categories = vec![
        term("en", "on-page-seo", "On-page SEO"),
        term("de", "on-page-seo", "On-Page-SEO"),
    ];
    index.tags = vec![term("en", "links", "Links"), term("en", "speed", "Speed")];
    index
}

#[tokio::test]
async fn the_lists_read_back_in_the_order_they_were_given() {
    let (_dir, db) = migrated().await;
    let site = db.site(&Slug::try_new("main").unwrap()).await.unwrap();
    let index = demo();
    db.rebuild(site, &index).await.unwrap();
    assert_eq!(db.authors(site).await.unwrap(), index.authors);
    assert_eq!(db.categories(site).await.unwrap(), index.categories);
    assert_eq!(db.tags(site).await.unwrap(), index.tags);
    db.close().await;
}

#[tokio::test]
async fn a_rebuild_replaces_the_lists_and_other_sites_keep_theirs() {
    let (_dir, db) = migrated().await;
    let main = db.site(&Slug::try_new("main").unwrap()).await.unwrap();
    let sister = db.site(&Slug::try_new("sister").unwrap()).await.unwrap();
    db.rebuild(main, &demo()).await.unwrap();
    db.rebuild(sister, &demo()).await.unwrap();

    let mut fewer = ContentIndex::default();
    fewer.authors = vec![author("en", "asif", "Asif Khan")];
    db.rebuild(main, &fewer).await.unwrap();

    assert_eq!(db.authors(main).await.unwrap(), fewer.authors);
    assert_eq!(db.categories(main).await.unwrap(), []);
    assert_eq!(db.tags(main).await.unwrap(), []);
    assert_eq!(db.authors(sister).await.unwrap(), demo().authors);
    db.close().await;
}

#[tokio::test]
async fn two_authors_with_one_slug_in_one_language_are_refused() {
    let (_dir, db) = migrated().await;
    let site = db.site(&Slug::try_new("main").unwrap()).await.unwrap();
    let mut twice = ContentIndex::default();
    let mut copy = author("en", "asif", "Asif");
    copy.path = "content/en/authors/asif-copy.md".to_owned();
    twice.authors = vec![author("en", "asif", "Asif"), copy];
    assert!(db.rebuild(site, &twice).await.is_err());
    db.close().await;
}
