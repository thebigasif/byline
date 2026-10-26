//! The content store contract, shown on the in-memory store with the demo
//! site's five English posts: filter, order, pages and lookups.

use chrono::{TimeZone, Utc};
use khalid_api::content_store::{ContentStore, MemoryContentStore, PostKey};
use khalid_api::query::{ListQuery, RawListQuery};
use khalid_api::resources::Post;
use khalid_core::{AbsoluteUrl, Locale, Slug};

/// A demo post: slug, title, publish month and day in 2026, tags.
type Demo = (
    &'static str,
    &'static str,
    (u32, u32),
    &'static [&'static str],
);

/// The demo site's English posts.
const DEMO: [Demo; 5] = [
    (
        "why-internal-links-matter",
        "Why internal links matter",
        (9, 14),
        &["seo", "links"],
    ),
    (
        "how-to-write-a-title-tag",
        "How to write a title tag",
        (9, 21),
        &["seo"],
    ),
    (
        "what-a-sitemap-does",
        "What a sitemap does, and what it does not",
        (9, 28),
        &["seo"],
    ),
    (
        "fixing-slow-pages",
        "Fixing slow pages, starting with the biggest win",
        (10, 5),
        &["speed"],
    ),
    (
        "planning-a-topic-cluster",
        "Planning a topic cluster in an afternoon",
        (10, 12),
        &["links"],
    ),
];

#[cfg(test)]
fn store() -> MemoryContentStore {
    let posts = DEMO.iter().map(|(slug, title, (month, day), tags)| {
        let url = format!("https://example.com/blog/{slug}/");
        let day = Utc.with_ymd_and_hms(2026, *month, *day, 9, 0, 0).unwrap();
        Post::new(
            &format!("{slug}-en"),
            Slug::try_new(slug).unwrap(),
            title,
            AbsoluteUrl::parse(&url).unwrap(),
            Locale::try_new("en").unwrap(),
            day,
        )
        .with_tags(tags.iter().map(|tag| (*tag).to_owned()).collect())
    });
    MemoryContentStore::new(posts.collect())
}

#[cfg(test)]
async fn slugs(pairs: &[(&str, &str)]) -> (Vec<String>, u64) {
    let mut raw = RawListQuery::default();
    for (name, value) in pairs {
        let value = Some((*value).to_owned());
        match *name {
            "filter" => raw.filter = value,
            "order" => raw.order = value,
            "limit" => raw.limit = value,
            _ => raw.page = value,
        }
    }
    let query = ListQuery::try_from(&raw).unwrap();
    let listed = store().posts(&query).await.unwrap();
    let slugs = listed
        .items()
        .iter()
        .map(|post| post.slug().to_string())
        .collect();
    (slugs, listed.total())
}

#[tokio::test]
async fn newest_first_by_default() {
    let (got, total) = slugs(&[]).await;
    assert_eq!(total, 5);
    assert_eq!(
        got,
        [
            "planning-a-topic-cluster",
            "fixing-slow-pages",
            "what-a-sitemap-does",
            "how-to-write-a-title-tag",
            "why-internal-links-matter",
        ]
    );
}

#[tokio::test]
async fn a_filter_keeps_what_matches_and_counts_it() {
    let (got, total) = slugs(&[("filter", "tag:links")]).await;
    assert_eq!(total, 2);
    assert_eq!(
        got,
        ["planning-a-topic-cluster", "why-internal-links-matter"]
    );
}

#[tokio::test]
async fn order_sorts_by_each_key_in_turn() {
    let (got, _) = slugs(&[("order", "title asc"), ("limit", "2")]).await;
    assert_eq!(got, ["fixing-slow-pages", "how-to-write-a-title-tag"]);
    let (got, _) = slugs(&[("order", "tags desc,published_at asc"), ("limit", "3")]).await;
    assert_eq!(
        got,
        [
            "fixing-slow-pages",
            "why-internal-links-matter",
            "how-to-write-a-title-tag"
        ]
    );
}

#[tokio::test]
async fn pages_cut_the_list_and_keep_the_total() {
    let (got, total) = slugs(&[("limit", "2"), ("page", "2")]).await;
    assert_eq!(
        (got.as_slice(), total),
        (
            &[
                "what-a-sitemap-does".to_owned(),
                "how-to-write-a-title-tag".to_owned()
            ][..],
            5
        )
    );
    let (got, total) = slugs(&[("limit", "2"), ("page", "9")]).await;
    assert_eq!((got.len(), total), (0, 5));
    let (got, _) = slugs(&[("limit", "all")]).await;
    assert_eq!(got.len(), 5);
}

#[tokio::test]
async fn one_post_by_id_or_slug() {
    let store = store();
    let by_slug = PostKey::Slug(Slug::try_new("fixing-slow-pages").unwrap());
    let found = store.post(&by_slug).await.unwrap().unwrap();
    assert_eq!(found.id(), "fixing-slow-pages-en");
    let by_id = PostKey::Id("what-a-sitemap-does-en".to_owned());
    assert!(store.post(&by_id).await.unwrap().is_some());
    let missing = PostKey::Slug(Slug::try_new("no-such-post").unwrap());
    assert_eq!(store.post(&missing).await.unwrap(), None);
}
