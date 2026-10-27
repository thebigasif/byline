//! The debug view for one address: which rule matched and why (KH-SEO-8).

#![cfg(test)]

use khalid_seo::{Matched, RedirectTable, UrlPath, read_redirects_csv};

const LIST: &str = "\
from,to,status,match
/blog/old-links/,/blog/internal-links/,301
/blog/internal-links/,/blog/why-internal-links-matter/,301
/blog/sale/,/blog/offers/,302
/blog/offers/,/blog/retired-offer/,301
/blog/retired-offer/,,410
/blog/court-order/,,451
/blog/serpdad/,https://serpdad.com/,307
/blog/\\d{4}/\\d{2}/(.+),/blog/$1,301,regex
";

fn table() -> RedirectTable {
    RedirectTable::from_entries(read_redirects_csv(LIST).unwrap()).unwrap()
}

fn explain(path: &str) -> String {
    table()
        .explain(&UrlPath::try_new(path).unwrap())
        .to_string()
}

#[test]
fn an_exact_match_shows_each_hop_and_the_one_step_a_visitor_gets() {
    let path = UrlPath::try_new("/blog/old-links/").unwrap();
    let explained = table().explain(&path);
    assert_eq!(explained.matched, Some(Matched::Exact { rule: 0 }));
    assert_eq!(explained.hops.len(), 2);
    assert_eq!(
        explained.to_string(),
        "\
/blog/old-links/ matches rule 0, the exact path /blog/old-links/.
1. /blog/old-links/ moves for good (301) to /blog/internal-links/, by rule 0.
2. /blog/internal-links/ moves for good (301) to /blog/why-internal-links-matter/, by rule 1.
A visitor gets one step: moves for good (301) to /blog/why-internal-links-matter/.
"
    );
}

#[test]
fn a_pattern_match_names_the_pattern_and_follows_into_exact_rules() {
    let path = UrlPath::try_new("/blog/2019/05/old-links/").unwrap();
    let explained = table().explain(&path);
    let pattern = r"/blog/\d{4}/\d{2}/(.+)".to_owned();
    assert_eq!(
        explained.matched,
        Some(Matched::Pattern { rule: 7, pattern })
    );
    assert_eq!(
        explained.to_string(),
        "\
/blog/2019/05/old-links/ matches rule 7, the pattern /blog/\\d{4}/\\d{2}/(.+).
1. /blog/2019/05/old-links/ moves for good (301) to /blog/old-links/, by rule 7.
2. /blog/old-links/ moves for good (301) to /blog/internal-links/, by rule 0.
3. /blog/internal-links/ moves for good (301) to /blog/why-internal-links-matter/, by rule 1.
A visitor gets one step: moves for good (301) to /blog/why-internal-links-matter/.
"
    );
}

#[test]
fn a_chain_ending_at_a_gone_page_says_so() {
    assert_eq!(
        explain("/blog/sale/"),
        "\
/blog/sale/ matches rule 2, the exact path /blog/sale/.
1. /blog/sale/ moves for now (302) to /blog/offers/, by rule 2.
2. /blog/offers/ moves for good (301) to /blog/retired-offer/, by rule 3.
3. /blog/retired-offer/ is gone (410), by rule 4.
A visitor gets one step: is gone (410).
"
    );
}

#[test]
fn every_other_answer_reads_plainly() {
    assert_eq!(
        explain("/blog/court-order/"),
        "\
/blog/court-order/ matches rule 5, the exact path /blog/court-order/.
1. /blog/court-order/ is blocked for a legal reason (451), by rule 5.
A visitor gets one step: is blocked for a legal reason (451).
"
    );
    assert!(explain("/blog/serpdad/").contains(
        "A visitor gets one step: moves for now, sent again as is (307) to https://serpdad.com/.\n"
    ));
}

#[test]
fn an_address_no_rule_matches_says_what_answers_instead() {
    let path = UrlPath::try_new("/blog/why-internal-links-matter/").unwrap();
    let explained = table().explain(&path);
    assert_eq!(explained.matched, None);
    assert_eq!(explained.hops, []);
    assert_eq!(explained.answer, None);
    assert_eq!(
        explained.to_string(),
        "/blog/why-internal-links-matter/ matches no redirect rule. \
         The page there answers, or the not-found page when there is none.\n"
    );
}
