//! JSON Feed 1.1 (task 1.13, KH-SEO-6), read back into strict types that
//! follow the specification at jsonfeed.org and refuse any unknown field.

#![cfg(test)]

mod common;

use common::{at, demo_site, internal_links_post, internal_links_post_de, locale};
use khalid_seo::{Feed, FeedScope, ShareImage};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct JsonFeed {
    version: String,
    title: String,
    home_page_url: Option<String>,
    feed_url: Option<String>,
    description: Option<String>,
    language: Option<String>,
    authors: Option<Vec<Author>>,
    items: Vec<Item>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Author {
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Item {
    id: String,
    url: Option<String>,
    title: Option<String>,
    content_html: Option<String>,
    summary: Option<String>,
    image: Option<String>,
    date_published: Option<String>,
    date_modified: Option<String>,
    authors: Option<Vec<Author>>,
    tags: Option<Vec<String>>,
}

fn demo_feed() -> Feed {
    let mut post = internal_links_post();
    post.content_html = String::from("<p>Link from <em>old</em> posts to new ones.</p>");
    post.share_image = Some(ShareImage::new(
        common::url("https://www.example.com/blog/media/internal-links.png"),
        1200,
        630,
        "A site map with arrows between pages",
    ));
    let mut german = internal_links_post_de();
    german.language = locale("en");
    Feed::for_pages(
        &demo_site(),
        &[post, german],
        &FeedScope::language(locale("en")),
        at(2026, 10, 26, 12),
    )
    .unwrap()
}

#[test]
fn the_demo_json_feed() {
    insta::assert_snapshot!(demo_feed().to_json().unwrap());
}

#[test]
fn the_json_feed_reads_back_into_the_specification_types() {
    let parsed: JsonFeed = serde_json::from_str(&demo_feed().to_json().unwrap()).unwrap();
    assert_eq!(parsed.version, "https://jsonfeed.org/version/1.1");
    assert_eq!(parsed.title, "Field Notes on SEO");
    assert_eq!(
        parsed.feed_url.as_deref(),
        Some("https://www.example.com/blog/feed.json")
    );
    assert_eq!(
        parsed.home_page_url.as_deref(),
        Some("https://www.example.com/blog/")
    );
    assert_eq!(parsed.language.as_deref(), Some("en"));
    assert!(parsed.description.is_some());
    assert_eq!(parsed.authors.unwrap()[0].name, "Field Notes on SEO");
    assert_eq!(parsed.items.len(), 1, "an undated post stays out");
    let item = &parsed.items[0];
    assert_eq!(
        item.id,
        "https://www.example.com/blog/why-internal-links-matter/"
    );
    assert_eq!(item.url.as_deref(), Some(item.id.as_str()));
    assert_eq!(item.title.as_deref(), Some("Why internal links matter"));
    assert!(
        item.content_html
            .as_deref()
            .unwrap()
            .starts_with("<p>Link from")
    );
    assert!(item.summary.is_some());
    assert!(
        item.image
            .as_deref()
            .unwrap()
            .ends_with("internal-links.png")
    );
    assert_eq!(item.date_published.as_deref(), Some("2026-09-14T08:00:00Z"));
    assert_eq!(item.date_modified.as_deref(), Some("2026-10-02T16:00:00Z"));
    assert_eq!(item.authors.as_ref().unwrap()[0].name, "Asif");
    assert_eq!(item.tags.as_deref(), Some(&["On-page SEO".to_owned()][..]));
}

#[test]
fn the_strict_types_refuse_a_feed_with_an_unknown_field() {
    let bad = r#"{"version":"https://jsonfeed.org/version/1.1","title":"T","items":[],"extra":1}"#;
    assert!(serde_json::from_str::<JsonFeed>(bad).is_err());
    let no_id =
        r#"{"version":"https://jsonfeed.org/version/1.1","title":"T","items":[{"url":"x"}]}"#;
    assert!(serde_json::from_str::<JsonFeed>(no_id).is_err());
}
