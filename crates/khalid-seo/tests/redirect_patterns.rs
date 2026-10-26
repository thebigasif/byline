//! Regex redirect rules (task 1.13, KH-SEO-8).

#![cfg(test)]

use khalid_seo::{
    Destination, Move, PatternRedirect, PatternRule, Redirect, RedirectPattern, RedirectRule,
    RedirectTable, SeoError, UrlPath,
};

fn path(text: &str) -> UrlPath {
    UrlPath::try_new(text).unwrap()
}

fn pattern(regex: &str, answer: PatternRedirect) -> PatternRule {
    PatternRule::new(RedirectPattern::try_new(regex).unwrap(), answer)
}

fn move_to(template: &str) -> PatternRedirect {
    PatternRedirect::move_to(Move::Permanent, template).unwrap()
}

fn demo_table() -> RedirectTable {
    let exact = vec![
        RedirectRule::new(
            path("/old-links/"),
            Redirect::Move {
                kind: Move::Permanent,
                to: Destination::Path(path("/why-internal-links-matter/")),
            },
        ),
        RedirectRule::new(path("/2019/05/keep-me/"), Redirect::Gone),
    ];
    RedirectTable::new(exact).unwrap().with_patterns(vec![
        pattern(r"/\d{4}/\d{2}/(.+)", move_to("/$1")),
        pattern(r"/docs/(.*)", move_to("https://docs.serpdad.com/$1")),
        pattern(r"/tmp/.*", PatternRedirect::Gone),
    ])
}

fn answer(table: &RedirectTable, from: &str) -> Option<(u16, Option<String>, usize)> {
    table.resolve(&path(from)).map(|hit| {
        (
            hit.status(),
            hit.location().map(ToString::to_string),
            hit.rule,
        )
    })
}

#[test]
fn a_dated_address_moves_to_its_slug_and_the_chain_collapses() {
    let table = demo_table();
    assert_eq!(
        answer(&table, "/2019/05/anchor-text/"),
        Some((301, Some("/anchor-text/".to_owned()), 2))
    );
    assert_eq!(
        answer(&table, "/2019/05/old-links/"),
        Some((301, Some("/why-internal-links-matter/".to_owned()), 2))
    );
}

#[test]
fn a_pattern_can_point_to_another_site_or_answer_gone() {
    let table = demo_table();
    assert_eq!(
        answer(&table, "/docs/setup/"),
        Some((301, Some("https://docs.serpdad.com/setup/".to_owned()), 3))
    );
    assert_eq!(answer(&table, "/tmp/upload-1/"), Some((410, None, 4)));
}

#[test]
fn an_exact_rule_wins_over_a_pattern() {
    assert_eq!(
        answer(&demo_table(), "/2019/05/keep-me/"),
        Some((410, None, 1))
    );
}

#[test]
fn a_pattern_must_match_the_whole_path() {
    assert_eq!(answer(&demo_table(), "/blog/tmp/x/"), None);
}

#[test]
fn a_broken_pattern_or_destination_is_refused() {
    assert!(matches!(
        RedirectPattern::try_new("/(unclosed"),
        Err(SeoError::RedirectPattern { .. })
    ));
    assert!(matches!(
        PatternRedirect::move_to(Move::Permanent, "no-slash/$1"),
        Err(SeoError::RedirectTemplate { .. })
    ));
}

#[test]
fn the_debug_view_lists_patterns_after_the_exact_rules() {
    insta::assert_snapshot!(demo_table().to_string());
}
