//! The redirect table (task 1.13, KH-SEO-8).

#![cfg(test)]

use khalid_core::AbsoluteUrl;
use khalid_seo::{Destination, Move, Redirect, RedirectRule, RedirectTable, SeoError, UrlPath};

fn path(text: &str) -> UrlPath {
    UrlPath::try_new(text).unwrap()
}

fn to(text: &str) -> Destination {
    Destination::Path(path(text))
}

fn rule(from: &str, redirect: Redirect) -> RedirectRule {
    RedirectRule::new(path(from), redirect)
}

fn moved(kind: Move, target: &str) -> Redirect {
    Redirect::Move {
        kind,
        to: to(target),
    }
}

fn demo_table() -> RedirectTable {
    RedirectTable::new(vec![
        rule("/old-links/", moved(Move::Permanent, "/internal-links/")),
        rule(
            "/internal-links/",
            moved(Move::Permanent, "/why-internal-links-matter/"),
        ),
        rule("/sale/", moved(Move::Found, "/offers/")),
        rule("/offers/", moved(Move::Temporary, "/offers-2026/")),
        rule("/retired-tool/", Redirect::Gone),
        rule(
            "/link-to-retired/",
            moved(Move::Permanent, "/retired-tool/"),
        ),
        rule("/court-order/", Redirect::LegalBlock),
        rule(
            "/serpdad/",
            Redirect::Move {
                kind: Move::Permanent,
                to: Destination::External(AbsoluteUrl::parse("https://serpdad.com/").unwrap()),
            },
        ),
    ])
    .unwrap()
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
fn each_status_answers_with_its_code_and_location() {
    let table = demo_table();
    assert_eq!(answer(&table, "/retired-tool/"), Some((410, None, 4)));
    assert_eq!(answer(&table, "/court-order/"), Some((451, None, 6)));
    assert_eq!(
        answer(&table, "/serpdad/"),
        Some((301, Some("https://serpdad.com/".to_owned()), 7))
    );
    assert_eq!(answer(&table, "/why-internal-links-matter/"), None);
}

#[test]
fn a_chain_collapses_into_one_hop_to_the_end() {
    let table = demo_table();
    assert_eq!(
        answer(&table, "/old-links/"),
        Some((301, Some("/why-internal-links-matter/".to_owned()), 0))
    );
}

#[test]
fn a_chain_with_a_temporary_hop_stays_temporary() {
    assert_eq!(
        answer(&demo_table(), "/sale/"),
        Some((302, Some("/offers-2026/".to_owned()), 2))
    );
}

#[test]
fn a_move_to_a_gone_page_answers_gone() {
    assert_eq!(
        answer(&demo_table(), "/link-to-retired/"),
        Some((410, None, 5))
    );
}

#[test]
fn a_loop_is_refused_when_the_table_is_built() {
    let error = RedirectTable::new(vec![
        rule("/a/", moved(Move::Permanent, "/b/")),
        rule("/b/", moved(Move::Permanent, "/a/")),
    ])
    .unwrap_err();
    assert!(
        matches!(error, SeoError::RedirectLoop { ref from } if from == "/a/"),
        "{error:?}"
    );
}

#[test]
fn a_source_listed_twice_is_refused() {
    let error = RedirectTable::new(vec![
        rule("/a/", moved(Move::Permanent, "/b/")),
        rule("/a/", Redirect::Gone),
    ])
    .unwrap_err();
    assert!(
        matches!(error, SeoError::RedirectDuplicate { ref from } if from == "/a/"),
        "{error:?}"
    );
}

#[test]
fn the_debug_view_prints_every_rule_and_its_collapsed_answer() {
    insta::assert_snapshot!(demo_table().to_string());
}
