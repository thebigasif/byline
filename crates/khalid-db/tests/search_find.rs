//! Searching the index: pages found by their terms in one language and one
//! site, ranked, with what each result shows, and never a page since gone.

use std::collections::BTreeSet;
use std::path::PathBuf;

use khalid_core::{Locale, Slug};
use khalid_db::{Database, SearchPage, SiteId};
use khalid_search::{Analyzer, Completion, Language, Query};

/// A migrated database in a fresh folder, and the site `main`.
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

/// The page of the post `slug` in `tag`, indexed with that language's rules.
#[cfg(test)]
fn page(tag: &str, slug: &str, title: &str, body: &str) -> SearchPage {
    let locale = Locale::try_new(tag).unwrap();
    let text = Analyzer::new(Language::of(&locale)).index(title, "", body);
    let path = format!("content/{tag}/posts/{slug}.md");
    let href = format!("/blog/{slug}/");
    SearchPage::new(&path, locale, &href, title, body, text)
}

/// The addresses a search for `typed` finds in `tag`, best first.
#[cfg(test)]
async fn found(db: &Database, site: SiteId, tag: &str, typed: &str) -> Vec<String> {
    let locale = Locale::try_new(tag).unwrap();
    let analyzer = Analyzer::new(Language::of(&locale));
    let query = Query::parse(&analyzer, typed, Completion::LastWordStarts).unwrap();
    let results = db.search(site, &locale, &query, 10).await.unwrap();
    results.hits.into_iter().map(|hit| hit.href).collect()
}

#[cfg(test)]
fn demo() -> Vec<SearchPage> {
    [
        ("en", "sitemaps", "What a sitemap does", "It lists pages."),
        (
            "en",
            "crawling",
            "How crawlers work",
            "They read a sitemap.",
        ),
        ("en", "anchors", "Anchor text", "Words that carry a link."),
        ("de", "sitemaps", "Die Sitemap", "Seiten und Links."),
    ]
    .into_iter()
    .map(|(tag, slug, title, body)| page(tag, slug, title, body))
    .collect()
}

#[tokio::test]
async fn written_pages_are_found_in_their_language_and_site_best_first() {
    let (_dir, db, site) = migrated().await;
    db.index_search(site, &demo(), &BTreeSet::new())
        .await
        .unwrap();

    assert_eq!(
        found(&db, site, "en", "sitemaps").await,
        ["/blog/sitemaps/", "/blog/crawling/"]
    );
    assert_eq!(found(&db, site, "de", "Sitemap").await, ["/blog/sitemaps/"]);
    assert_eq!(found(&db, site, "en", "anch").await, ["/blog/anchors/"]);
    let nothing = Vec::<String>::new();
    assert_eq!(found(&db, site, "en", "unicorn").await, nothing);
    let sister = db.site(&Slug::try_new("sister").unwrap()).await.unwrap();
    assert_eq!(found(&db, sister, "en", "sitemap").await, nothing);
    db.close().await;
}

#[tokio::test]
async fn a_hit_carries_what_a_result_shows_and_the_total_counts_every_match() {
    let (_dir, db, site) = migrated().await;
    db.index_search(site, &demo(), &BTreeSet::new())
        .await
        .unwrap();
    let english = Locale::try_new("en").unwrap();
    let analyzer = Analyzer::new(Language::English);
    let query = Query::parse(&analyzer, "sitemap", Completion::WholeWords).unwrap();

    let results = db.search(site, &english, &query, 1).await.unwrap();
    assert_eq!(results.total, 2);
    let [hit] = results.hits.as_slice() else {
        panic!("one hit: {:?}", results.hits);
    };
    assert_eq!(hit.title, "What a sitemap does");
    assert_eq!(hit.body, "It lists pages.");
    db.close().await;
}

#[tokio::test]
async fn a_changed_page_replaces_its_terms_and_a_removed_page_is_gone() {
    let (_dir, db, site) = migrated().await;
    db.index_search(site, &demo(), &BTreeSet::new())
        .await
        .unwrap();

    let edited = page("en", "crawling", "How robots work", "They read robots.txt.");
    let removed = BTreeSet::from(["content/en/posts/anchors.md".to_owned()]);
    db.index_search(site, &[edited], &removed).await.unwrap();

    assert_eq!(found(&db, site, "en", "sitemap").await, ["/blog/sitemaps/"]);
    assert_eq!(found(&db, site, "en", "robots").await, ["/blog/crawling/"]);
    assert_eq!(found(&db, site, "en", "anchor").await, Vec::<String>::new());
    db.close().await;
}
