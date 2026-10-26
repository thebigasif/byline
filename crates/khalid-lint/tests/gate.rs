//! The publish gate the server calls, and the JSON every surface shows.

use chrono::{NaiveDate, TimeZone, Utc};
use khalid_core::{FixedClock, Locale, Slug};
use khalid_lint::{LinkKind, LintConfig, LintLink, LintPost, LintSite, Linter, Verdict};

#[cfg(test)]
fn linter(lint_toml: &str) -> Linter {
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 10, 26, 9, 0, 0).unwrap());
    Linter::new(&LintConfig::parse(lint_toml).unwrap(), &clock).unwrap()
}

/// A post that passes every block but has no description, a warning.
#[cfg(test)]
fn site_with_post(date: Option<NaiveDate>) -> LintSite {
    let mut post = LintPost::new(
        Locale::try_new("en").unwrap(),
        Slug::try_new("why-internal-links-matter").unwrap(),
    );
    post.title = Some("Why internal links matter".to_owned());
    post.author = Some("Amira Haddad".to_owned());
    post.date = date;
    post.word_count = 640;
    post.links.push(LintLink::new(
        LinkKind::Internal,
        "/blog/link-text/",
        "link text",
    ));
    post.links.push(LintLink::new(
        LinkKind::External,
        "https://example.org/",
        "a source",
    ));
    let mut site = LintSite::default();
    site.posts.push(post);
    site
}

#[test]
fn the_gate_publishes_a_post_with_only_warnings() {
    let site = site_with_post(NaiveDate::from_ymd_opt(2026, 10, 1));
    let verdict = linter("").gate(&site.posts[0], &site);
    assert!(matches!(verdict, Verdict::Publish { .. }), "{verdict:?}");
    assert!(verdict.allows_publishing());
    let rules: Vec<_> = verdict
        .report()
        .findings()
        .iter()
        .map(|f| f.rule().as_str())
        .collect();
    assert_eq!(
        rules,
        ["no-description"],
        "warnings travel with the verdict"
    );
}

#[test]
fn the_gate_refuses_a_post_with_a_block_and_says_why() {
    let site = site_with_post(None);
    let verdict = linter("").gate(&site.posts[0], &site);
    assert!(matches!(verdict, Verdict::Refuse { .. }), "{verdict:?}");
    assert!(!verdict.allows_publishing());
    assert!(verdict.report().blocks());
}

#[test]
fn lint_toml_can_open_the_gate() {
    let site = site_with_post(None);
    let verdict = linter("[rules]\nno-date = \"warn\"\n").gate(&site.posts[0], &site);
    assert!(verdict.allows_publishing());
}

#[test]
fn a_report_serialises_with_the_message_and_its_place() {
    let site = site_with_post(None);
    let report = linter("").lint_post(&site.posts[0], &site);
    let json = serde_json::to_value(&report).unwrap();
    assert_eq!(json["blocks"], true);
    let first = &json["findings"][0];
    assert_eq!(first["rule"], "no-date");
    assert_eq!(first["severity"], "block");
    assert_eq!(first["locale"], "en");
    assert_eq!(first["slug"], "why-internal-links-matter");
    assert_eq!(first["subject"], serde_json::Value::Null);
    assert_eq!(first["what"], "This post has no date.");
    assert_eq!(first["fix"], "Add `date:` to the header.");
    assert_eq!(first["guide"], "post-dates");
    assert_eq!(first["suggestions"], serde_json::json!([]));
}
