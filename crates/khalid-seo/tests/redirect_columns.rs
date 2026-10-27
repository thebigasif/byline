//! A redirect as four plain columns, the form a CSV row and a database row
//! share (KH-SEO-8).

#![cfg(test)]

use khalid_seo::{Matching, RedirectColumns, RedirectEntry, SeoError};

#[test]
fn columns_give_back_the_rule_they_came_from() {
    let rows = [
        RedirectColumns::new("/blog/sale/", Some("/blog/offers/"), 302, Matching::Exact),
        RedirectColumns::new(
            "/blog/serpdad/",
            Some("https://serpdad.com/"),
            307,
            Matching::Exact,
        ),
        RedirectColumns::new("/blog/court-order/", None, 451, Matching::Exact),
        RedirectColumns::new(
            "/blog/(\\d+)/",
            Some("/blog/post-$1/"),
            301,
            Matching::Regex,
        ),
        RedirectColumns::new("/blog/tmp/.*", None, 410, Matching::Regex),
    ];
    for columns in rows {
        let entry = RedirectEntry::from_columns(&columns).unwrap();
        assert_eq!(entry.columns(), columns);
    }
}

#[test]
fn a_status_and_destination_that_do_not_fit_are_refused() {
    let cases = [
        (
            308,
            Some("/blog/b/"),
            "has a status other than 301, 302, 307, 410 or 451",
        ),
        (
            301,
            None,
            "moves to nowhere: a 301, 302 or 307 needs a to address",
        ),
        (
            301,
            Some(" "),
            "moves to nowhere: a 301, 302 or 307 needs a to address",
        ),
        (
            410,
            Some("/blog/b/"),
            "has a to address, which a 410 or 451 cannot use",
        ),
    ];
    for (status, to, problem) in cases {
        let columns = RedirectColumns::new("/blog/a/", to, status, Matching::Exact);
        let from = "/blog/a/".to_owned();
        let error = RedirectEntry::from_columns(&columns).unwrap_err();
        assert_eq!(
            error,
            SeoError::RedirectColumns { from, problem },
            "{status} {to:?}"
        );
    }
}

#[test]
fn match_names_read_back_in_any_case() {
    assert_eq!(Matching::from_name(" Regex "), Some(Matching::Regex));
    assert_eq!(
        Matching::from_name(Matching::Exact.as_str()),
        Some(Matching::Exact)
    );
    assert_eq!(Matching::from_name("glob"), None);
}
