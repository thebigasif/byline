//! The demo site's English posts and one entry per other collection,
//! shared by the API tests.

use chrono::{TimeZone, Utc};
use khalid_api::content_store::{MemoryContentStore, SiteResources};
use khalid_api::resources::{Author, Bodies, Collection, Post};
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

/// One entry from each other collection of the demo site.
const OTHERS: [(Collection, &str, &str); 4] = [
    (Collection::Pages, "about", "About this blog"),
    (Collection::Glossary, "anchor-text", "Anchor text"),
    (
        Collection::CaseStudies,
        "a-bakery-doubles-its-traffic",
        "A bakery doubles its traffic",
    ),
    (
        Collection::Changelog,
        "2026-10-release",
        "October 2026 release",
    ),
];

/// A document with a one-line body, published on `month` and `day` 2026.
#[cfg(test)]
fn document(slug: &str, title: &str, (month, day): (u32, u32)) -> Post {
    let url = format!("https://example.com/blog/{slug}/");
    let day = Utc.with_ymd_and_hms(2026, month, day, 9, 0, 0).unwrap();
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
    .with_bodies(Bodies::new(&html, &text, &text))
}

/// A store holding the demo posts and one entry in each other collection.
#[cfg(test)]
pub(crate) fn store() -> MemoryContentStore {
    let posts = DEMO.iter().map(|(slug, title, day, tags)| {
        let tags = tags.iter().map(|tag| (*tag).to_owned()).collect();
        (
            Collection::Posts,
            document(slug, title, *day).with_tags(tags),
        )
    });
    let others = OTHERS
        .iter()
        .map(|(collection, slug, title)| (*collection, document(slug, title, (10, 1))));
    MemoryContentStore::new(posts.chain(others).collect()).with_site(site())
}

/// The demo site's authors in both languages.
#[cfg(test)]
fn site() -> SiteResources {
    let (en, de) = (
        Locale::try_new("en").unwrap(),
        Locale::try_new("de").unwrap(),
    );
    let slug = |slug: &str| Slug::try_new(slug).unwrap();
    let url = |url: &str| AbsoluteUrl::parse(url).unwrap();
    SiteResources::default().with_authors(vec![
        Author::new(slug("asif"), "Asif", en.clone())
            .with_role("Founder of SerpDad")
            .with_url(url("https://example.com/blog/authors/asif/"))
            .with_same_as(vec![url("https://serpdad.com/")]),
        Author::new(slug("lena-vogt"), "Lena Vogt", en),
        Author::new(slug("asif"), "Asif", de).with_role("Head of SerpDad"),
    ])
}
