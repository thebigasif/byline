//! A post as the admin's posts table lists it, and the filters it answers
//! (BL-API-5, KH-API-2).

use chrono::{TimeZone, Utc};
use khalid_api::query::{FieldValues, Filter};
use khalid_api::resources::{
    AdminPost, IndexStatus, LintCounts, PostRow, PostSettings, ReviewState,
};
use khalid_core::Slug;
use serde_json::json;

#[cfg(test)]
fn draft(slug: &str) -> AdminPost {
    let at = Utc.with_ymd_and_hms(2026, 10, 20, 8, 30, 0).unwrap();
    AdminPost::new(
        slug,
        Slug::try_new(slug).unwrap(),
        "Choosing anchor text",
        "Text.",
        at,
    )
}

#[cfg(test)]
fn live(slug: &str, settings: PostSettings) -> AdminPost {
    let at = Utc.with_ymd_and_hms(2026, 9, 14, 0, 0, 0).unwrap();
    draft(slug).with_settings(settings).published(at)
}

#[test]
fn a_row_has_the_columns_and_no_body() {
    let mut settings = PostSettings::default();
    settings.review_state = ReviewState::InReview;
    let post = draft("anchor-text")
        .with_settings(settings)
        .with_updated_by("Asif");
    let json = serde_json::to_value(PostRow::new(&post, LintCounts::new(2, 1))).unwrap();
    let want = json!({
        "id": "anchor-text", "slug": "anchor-text", "title": "Choosing anchor text",
        "author": null, "status": "draft", "review_state": "in_review", "featured": false,
        "published_at": null, "updated_at": "2026-10-20T08:30:00Z", "updated_by": "Asif",
        "lint": {"blocks": 2, "warnings": 1}, "index_status": "not_live",
    });
    assert_eq!(json, want);
}

#[test]
fn the_index_status_follows_status_robots_and_canonical() {
    let mut noindex = PostSettings::default();
    noindex.robots = Some("noindex, follow".parse().unwrap());
    let mut credited = PostSettings::default();
    credited.canonical = Some("https://example.com/original/".parse().unwrap());
    let cases = [
        (draft("a"), IndexStatus::NotLive),
        (live("b", PostSettings::default()), IndexStatus::Indexable),
        (live("c", noindex), IndexStatus::Noindex),
        (live("d", credited), IndexStatus::CanonicalElsewhere),
    ];
    for (post, want) in cases {
        assert_eq!(IndexStatus::of(&post), want, "{}", post.id());
        let json = serde_json::to_value(want).unwrap();
        assert_eq!(json, want.as_str());
    }
}

#[test]
fn the_shared_filter_grammar_reaches_every_column() {
    let mut featured = PostSettings::default();
    featured.featured = true;
    featured.review_state = ReviewState::Ready;
    let rows = [
        PostRow::new(&draft("weak"), LintCounts::new(3, 0)),
        PostRow::new(&live("strong", featured), LintCounts::new(0, 2)),
    ];
    let cases = [
        ("lint.blocks:>0", "weak"),
        ("lint.warnings:2", "strong"),
        ("status:draft", "weak"),
        ("review_state:ready+featured:true", "strong"),
        ("index_status:not_live", "weak"),
        ("published_at:null", "weak"),
        ("published_at:>'2026-09-01'", "strong"),
        ("author:null+slug:~'wea'", "weak"),
    ];
    for (filter, want) in cases {
        let filter: Filter = filter.parse().unwrap();
        let found: Vec<&str> = rows
            .iter()
            .filter(|row| filter.matches(*row))
            .map(PostRow::id)
            .collect();
        assert_eq!(found, [want], "{filter}");
    }
    assert!(rows[0].values("markdown").is_empty(), "no body in a row");
}
