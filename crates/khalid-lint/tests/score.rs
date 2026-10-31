//! The score out of 100 that sums up a post's findings for the dashboard,
//! and points at them instead of replacing them (KH-LNT-4, plan D-10).

use chrono::{TimeZone, Utc};
use khalid_core::{FixedClock, Locale, Slug};
use khalid_lint::{Image, LintConfig, LintPost, LintSite, Linter, Score};

/// The clean fixture: a post with no findings.
#[cfg(test)]
fn clean() -> LintPost {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/clean/post.toml");
    toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[cfg(test)]
fn score(post: &LintPost) -> Score {
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 10, 26, 9, 0, 0).unwrap());
    let linter = Linter::new(&LintConfig::default(), &clock).unwrap();
    let mut site = LintSite::default();
    site.posts.push(post.clone());
    linter.lint_post(post, &site).score(post)
}

#[cfg(test)]
fn lost(score: &Score) -> Vec<(&str, u8)> {
    score
        .lost()
        .iter()
        .map(|loss| (loss.rule().as_str(), loss.points()))
        .collect()
}

#[test]
fn a_post_with_no_findings_scores_100() {
    let score = score(&clean());
    assert_eq!(score.value(), 100);
    assert_eq!(score.lost(), []);
}

#[test]
fn each_warning_costs_five_points_and_names_its_rule() {
    let mut post = clean();
    post.focus_keyword = None;
    post.description = None;
    let score = score(&post);
    assert_eq!(score.value(), 90);
    assert_eq!(
        lost(&score),
        [("no-description", 5), ("no-focus-keyword", 5)]
    );
}

#[test]
fn a_rule_costs_points_once_however_often_it_fires() {
    let mut post = clean();
    post.images = vec![
        Image::new("/media/one.png", None),
        Image::new("/media/two.png", None),
    ];
    let score = score(&post);
    assert_eq!(score.value(), 95);
    assert_eq!(lost(&score), [("missing-alt", 5)]);
}

#[test]
fn a_post_the_gate_refuses_scores_at_most_40() {
    let mut post = clean();
    post.date = None;
    let score = score(&post);
    assert_eq!(score.value(), 40);
    assert_eq!(lost(&score), [("no-date", 30)]);
}

#[test]
fn the_score_never_drops_below_0() {
    let post = LintPost::new(
        Locale::try_new("en").unwrap(),
        Slug::try_new("empty").unwrap(),
    );
    assert_eq!(score(&post).value(), 0);
}

#[test]
fn findings_about_other_posts_do_not_count() {
    let other = LintPost::new(
        Locale::try_new("en").unwrap(),
        Slug::try_new("empty").unwrap(),
    );
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 10, 26, 9, 0, 0).unwrap());
    let linter = Linter::new(&LintConfig::default(), &clock).unwrap();
    let mut site = LintSite::default();
    site.posts.push(other.clone());
    let report = linter.lint_post(&other, &site);
    assert_eq!(report.score(&clean()).value(), 100);
}

#[test]
fn the_score_serialises_with_what_it_lost() {
    let mut post = clean();
    post.date = None;
    let json = serde_json::to_value(score(&post)).unwrap();
    assert_eq!(
        json,
        serde_json::json!({"value": 40, "lost": [{"rule": "no-date", "points": 30}]})
    );
}

#[cfg(test)]
fn linter() -> Linter {
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 10, 26, 9, 0, 0).unwrap());
    Linter::new(&LintConfig::default(), &clock).unwrap()
}

#[test]
fn a_report_on_one_post_carries_its_score() {
    let mut post = clean();
    post.date = None;
    let mut site = LintSite::default();
    site.posts.push(post.clone());
    let report = linter().lint_post(&post, &site);
    assert_eq!(report.post_score().map(Score::value), Some(40));
    let json = serde_json::to_value(&report).unwrap();
    assert_eq!(json["score"]["value"], 40);
    assert_eq!(json["score"]["lost"][0]["rule"], "no-date");
}

#[test]
fn a_report_on_a_site_scores_each_post_once_in_site_order() {
    let mut undated = clean();
    undated.date = None;
    let mut fine = clean();
    fine.slug = Slug::try_new("another-post").unwrap();
    let mut site = LintSite::default();
    site.posts.push(undated);
    site.posts.push(fine);
    let report = linter().lint_site(&site);
    assert_eq!(report.post_score(), None);
    assert_eq!(serde_json::to_value(&report).unwrap().get("score"), None);
    let scores: Vec<(&str, u8)> = report
        .scores(&site.posts)
        .iter()
        .map(|(post, score)| (post.slug.as_str(), score.value()))
        .collect();
    let one_by_one: Vec<(&str, u8)> = site
        .posts
        .iter()
        .map(|post| (post.slug.as_str(), report.score(post).value()))
        .collect();
    assert_eq!(scores, one_by_one);
    assert_eq!(scores.len(), 2);
    assert_eq!(scores.first().map(|(_, value)| *value), Some(40));
}
