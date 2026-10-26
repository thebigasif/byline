//! The content store contract, shown on the in-memory store with the demo
//! site's five English posts and its authors: filter, order, pages, lookups
//! and the site's small lists.

mod common;

use common::store;
use khalid_api::content_store::{ContentStore, MemoryContentStore, PostKey, SiteResources};
use khalid_api::query::{ListQuery, RawListQuery};
use khalid_api::resources::Collection;
use khalid_core::Slug;

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
    let listed = store().list(Collection::Posts, &query).await.unwrap();
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
    let found = store
        .find(Collection::Posts, &by_slug)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.id(), "fixing-slow-pages-en");
    let by_id = PostKey::Id("what-a-sitemap-does-en".to_owned());
    assert!(
        store
            .find(Collection::Posts, &by_id)
            .await
            .unwrap()
            .is_some()
    );
    let missing = PostKey::Slug(Slug::try_new("no-such-post").unwrap());
    assert_eq!(store.find(Collection::Posts, &missing).await.unwrap(), None);
}

#[tokio::test]
async fn each_collection_keeps_its_own_entries() {
    let store = store();
    let query = ListQuery::try_from(&RawListQuery::default()).unwrap();
    let pages = store.list(Collection::Pages, &query).await.unwrap();
    let slugs: Vec<String> = pages
        .items()
        .iter()
        .map(|page| page.slug().to_string())
        .collect();
    assert_eq!((slugs, pages.total()), (vec!["about".to_owned()], 1));
    let about = PostKey::Slug(Slug::try_new("about").unwrap());
    assert_eq!(store.find(Collection::Posts, &about).await.unwrap(), None);
    let fresh = MemoryContentStore::new(Vec::new());
    assert_eq!(fresh.site().await.unwrap(), SiteResources::default());
    assert!(
        store
            .find(Collection::Pages, &about)
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn the_site_lists_come_whole_in_api_shape() {
    let site = store().site().await.unwrap();
    let authors = serde_json::to_value(site.authors()).unwrap();
    let ids: Vec<&str> = authors
        .as_array()
        .unwrap()
        .iter()
        .map(|author| author["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["en.asif", "en.lena-vogt", "de.asif"]);
    assert_eq!(
        authors[0],
        serde_json::json!({
            "id": "en.asif",
            "slug": "asif",
            "name": "Asif",
            "language": "en",
            "role": "Founder of SerpDad",
            "bio": null,
            "url": "https://example.com/blog/authors/asif/",
            "same_as": ["https://serpdad.com/"],
        })
    );
    assert_eq!(authors[1]["role"], serde_json::Value::Null);
    assert_eq!(authors[1]["same_as"], serde_json::json!([]));
}
