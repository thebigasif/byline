//! The search index: pages written at publish time, each with its terms,
//! and replaced or removed when their files change.

use std::collections::BTreeSet;
use std::path::PathBuf;

use khalid_core::{Locale, Slug};
use khalid_db::{Database, SearchPage, SiteId};
use khalid_search::{Analyzer, Language};
use sqlx::ConnectOptions;
use sqlx::sqlite::{SqliteConnectOptions, SqliteConnection};

/// A migrated database with the site `main`, and a plain connection to it,
/// so the checks do not depend on the code they check.
#[cfg(test)]
async fn migrated() -> (tempfile::TempDir, Database, SiteId, SqliteConnection) {
    let dir = tempfile::tempdir().unwrap();
    let mut path = PathBuf::from(dir.path());
    path.push("site.db");
    let db = Database::open(&path).await.unwrap();
    db.migrate().await.unwrap();
    let site = db.site(&Slug::try_new("main").unwrap()).await.unwrap();
    let connection = SqliteConnectOptions::new()
        .filename(&path)
        .connect()
        .await
        .unwrap();
    (dir, db, site, connection)
}

/// The English post `slug`, with `title` and `body`.
#[cfg(test)]
fn page(slug: &str, title: &str, body: &str) -> SearchPage {
    let locale = Locale::try_new("en").unwrap();
    let text = Analyzer::new(Language::English).index(title, "", body);
    let path = format!("content/en/posts/{slug}.md");
    SearchPage::new(&path, locale, &format!("/blog/{slug}/"), title, body, text)
}

/// One stored term: the term, its page's address, and its counts per field.
type Stored = (String, String, i64, i64, i64);

/// Every stored term, in order.
#[cfg(test)]
async fn terms(connection: &mut SqliteConnection) -> Vec<Stored> {
    sqlx::query_as(
        "SELECT t.term, p.href, t.in_title, t.in_headings, t.in_body \
         FROM search_terms t JOIN search_pages p ON p.id = t.page_id \
         ORDER BY t.term, p.href",
    )
    .fetch_all(connection)
    .await
    .unwrap()
}

#[tokio::test]
async fn a_written_page_stores_each_term_with_its_counts_per_field() {
    let (_dir, db, site, mut connection) = migrated().await;
    let pages = [page("sitemaps", "Sitemaps", "A sitemap lists pages.")];
    db.index_search(site, &pages, &BTreeSet::new())
        .await
        .unwrap();

    let stored: Stored = sqlx::query_as(
        "SELECT href, title, title_words, heading_words, body_words FROM search_pages",
    )
    .fetch_one(&mut connection)
    .await
    .unwrap();
    let href = "/blog/sitemaps/".to_owned();
    assert_eq!(stored, (href.clone(), "Sitemaps".into(), 1, 0, 4));
    let found = terms(&mut connection).await;
    assert!(
        found.contains(&("sitemap".into(), href.clone(), 1, 0, 1)),
        "{found:?}"
    );
    assert!(found.contains(&("page".into(), href, 0, 0, 1)), "{found:?}");
    db.close().await;
}

#[tokio::test]
async fn a_changed_page_replaces_its_terms_and_a_removed_page_takes_its_terms_along() {
    let (_dir, db, site, mut connection) = migrated().await;
    let pages = [
        page("crawling", "Crawlers", "They read a sitemap."),
        page("anchors", "Anchors", "Words."),
    ];
    db.index_search(site, &pages, &BTreeSet::new())
        .await
        .unwrap();

    let edited = page("crawling", "Robots", "Robots");
    let removed = BTreeSet::from(["content/en/posts/anchors.md".to_owned()]);
    db.index_search(site, &[edited], &removed).await.unwrap();

    let href = "/blog/crawling/".to_owned();
    assert_eq!(
        terms(&mut connection).await,
        [("robot".into(), href, 1, 0, 1)]
    );
    db.close().await;
}
