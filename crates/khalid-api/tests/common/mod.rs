//! The demo site's five English posts, shared by the API tests.

use chrono::{TimeZone, Utc};
use khalid_api::content_store::MemoryContentStore;
use khalid_api::resources::{Bodies, Post};
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

/// A store holding the demo posts, each with a one-line body.
#[cfg(test)]
pub(crate) fn store() -> MemoryContentStore {
    let posts = DEMO.iter().map(|(slug, title, (month, day), tags)| {
        let url = format!("https://example.com/blog/{slug}/");
        let day = Utc.with_ymd_and_hms(2026, *month, *day, 9, 0, 0).unwrap();
        let html = format!("<p>{title}.</p>");
        let text = format!("{title}.");
        Post::new(
            &format!("{slug}-en"),
            Slug::try_new(slug).unwrap(),
            title,
            AbsoluteUrl::parse(&url).unwrap(),
            Locale::try_new("en").unwrap(),
            day,
        )
        .with_tags(tags.iter().map(|tag| (*tag).to_owned()).collect())
        .with_bodies(Bodies::new(&html, &text, &text))
    });
    MemoryContentStore::new(posts.collect())
}
