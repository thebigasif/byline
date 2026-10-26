//! A post as the API sends it, and the fields a filter can test.

use chrono::{DateTime, TimeZone, Utc};
use khalid_api::query::{FieldValues, Filter, Literal};
use khalid_api::resources::{Bodies, Post};
use khalid_core::{AbsoluteUrl, Locale, Slug};
use serde_json::json;

#[cfg(test)]
fn at(text: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(text)
        .unwrap()
        .with_timezone(&Utc)
}

#[cfg(test)]
fn post() -> Post {
    Post::new(
        "why-internal-links-matter-en",
        Slug::try_new("why-internal-links-matter").unwrap(),
        "Why internal links matter",
        AbsoluteUrl::parse("https://example.com/blog/why-internal-links-matter/").unwrap(),
        Locale::try_new("en").unwrap(),
        at("2026-03-14T09:00:00Z"),
    )
    .with_excerpt("Links tell readers and search engines what matters.")
    .with_tags(vec!["seo".to_owned(), "links".to_owned()])
    .with_authors(vec!["asif".to_owned()])
    .featured()
    .with_updated_at(at("2026-03-20T10:30:00Z"))
    .with_bodies(Bodies::new(
        "<p>Links matter.</p>",
        "Links matter.",
        "Links matter.",
    ))
}

#[test]
fn a_post_serialises_in_the_api_shape() {
    let json = serde_json::to_value(post()).unwrap();
    assert_eq!(
        json,
        json!({
            "id": "why-internal-links-matter-en",
            "slug": "why-internal-links-matter",
            "title": "Why internal links matter",
            "url": "https://example.com/blog/why-internal-links-matter/",
            "language": "en",
            "excerpt": "Links tell readers and search engines what matters.",
            "featured": true,
            "tags": ["seo", "links"],
            "authors": ["asif"],
            "published_at": "2026-03-14T09:00:00Z",
            "updated_at": "2026-03-20T10:30:00Z",
            "html": "<p>Links matter.</p>",
            "markdown": "Links matter.",
            "plaintext": "Links matter.",
        })
    );
}

#[test]
fn a_new_post_has_plain_defaults() {
    let plain = Post::new(
        "glossary-anchor-text-en",
        Slug::try_new("anchor-text").unwrap(),
        "Anchor text",
        AbsoluteUrl::parse("https://example.com/blog/anchor-text/").unwrap(),
        Locale::try_new("en").unwrap(),
        Utc.with_ymd_and_hms(2026, 1, 2, 3, 4, 5).unwrap(),
    );
    let json = serde_json::to_value(plain).unwrap();
    assert_eq!(json["excerpt"], json!(null));
    assert_eq!(json["featured"], json!(false));
    assert_eq!(json["tags"], json!([]));
    assert_eq!(json["updated_at"], json["published_at"]);
    assert_eq!(json.get("html"), None);
}

#[test]
fn every_filter_field_has_values() {
    let post = post();
    let text = |value: &str| Literal::Text(value.to_owned());
    let cases = [
        ("id", vec![text("why-internal-links-matter-en")]),
        ("slug", vec![text("why-internal-links-matter")]),
        ("title", vec![text("Why internal links matter")]),
        ("language", vec![text("en")]),
        ("featured", vec![Literal::Bool(true)]),
        ("tag", vec![text("seo"), text("links")]),
        ("tags", vec![text("seo"), text("links")]),
        ("author", vec![text("asif")]),
        ("authors", vec![text("asif")]),
        ("published_at", vec![text("2026-03-14T09:00:00Z")]),
        ("updated_at", vec![text("2026-03-20T10:30:00Z")]),
    ];
    assert_eq!(Post::FILTER_FIELDS.len(), cases.len());
    for (field, want) in cases {
        assert!(Post::FILTER_FIELDS.contains(&field), "{field}");
        assert_eq!(post.values(field), want, "{field}");
    }
    assert_eq!(post.values("colour"), []);
}

#[test]
fn the_examples_from_the_api_standard_apply_to_a_post() {
    let holds = |text: &str| text.parse::<Filter>().unwrap().matches(&post());
    assert!(holds("tag:seo+featured:true"));
    assert!(holds("tag:[seo,links]"));
    assert!(holds("tag:-draft"));
    assert!(holds("published_at:>'2026-01-01'"));
    assert!(!holds("published_at:>'2026-06-01'"));
}
