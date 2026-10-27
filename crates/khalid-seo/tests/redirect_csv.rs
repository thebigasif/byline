//! Redirect lists as CSV, for import and export (KH-SEO-8).

#![cfg(test)]

use khalid_seo::{RedirectEntry, RedirectTable, SeoError, UrlPath, read_redirects_csv};

/// One row of each kind, in the columns the importers write.
const EVERY_KIND: &str = "\
from,to,status,match
/blog/old-links/,/blog/why-internal-links-matter/,301
/blog/sale/,/blog/offers/,302,exact
/blog/offers/,/blog/offers-2026/,307,exact
/blog/retired-tool/,,410,exact
/blog/court-order/,,451,exact
/blog/serpdad/,https://serpdad.com/,301,exact
\"/blog/\\d{4}/\\d{2}/(.+)\",/blog/$1,301,regex
/blog/tmp/.*,,410,regex
";

fn answer(table: &RedirectTable, from: &str) -> Option<(u16, Option<String>)> {
    let path = UrlPath::try_new(from).unwrap();
    table
        .resolve(&path)
        .map(|hit| (hit.status(), hit.location().map(ToString::to_string)))
}

#[test]
fn every_status_and_both_match_kinds_are_read() {
    let entries = read_redirects_csv(EVERY_KIND).unwrap();
    assert_eq!(entries.len(), 8);
    assert!(matches!(entries[0], RedirectEntry::Exact(_)));
    assert!(matches!(entries[6], RedirectEntry::Pattern(_)));
    let table = RedirectTable::from_entries(entries).unwrap();
    let to = |path: &str| Some(path.to_owned());
    assert_eq!(
        answer(&table, "/blog/old-links/"),
        Some((301, to("/blog/why-internal-links-matter/")))
    );
    assert_eq!(
        answer(&table, "/blog/sale/"),
        Some((302, to("/blog/offers-2026/")))
    );
    assert_eq!(
        answer(&table, "/blog/offers/"),
        Some((307, to("/blog/offers-2026/")))
    );
    assert_eq!(answer(&table, "/blog/retired-tool/"), Some((410, None)));
    assert_eq!(answer(&table, "/blog/court-order/"), Some((451, None)));
    assert_eq!(
        answer(&table, "/blog/serpdad/"),
        Some((301, to("https://serpdad.com/")))
    );
    assert_eq!(
        answer(&table, "/blog/2019/05/old-links/"),
        Some((301, to("/blog/why-internal-links-matter/")))
    );
    assert_eq!(answer(&table, "/blog/tmp/draft/"), Some((410, None)));
}

#[test]
fn quoted_fields_crlf_endings_and_blank_lines_are_read() {
    let text = "From,To,Status,Match\r\n\r\n\"/blog/a,b/\",\"/blog/say-\"\"hi\"\"/\",301,exact\r\n";
    let table = RedirectTable::from_entries(read_redirects_csv(text).unwrap()).unwrap();
    assert_eq!(
        answer(&table, "/blog/a,b/"),
        Some((301, Some("/blog/say-\"hi\"/".to_owned())))
    );
}

#[test]
fn a_file_without_the_header_is_refused() {
    let error = read_redirects_csv("/blog/a/,/blog/b/,301\n").unwrap_err();
    assert_eq!(
        error,
        SeoError::RedirectCsvRow {
            line: 1,
            problem: "is not the header from,to,status,match",
        }
    );
}

#[test]
fn each_bad_row_is_refused_with_its_line() {
    let cases = [
        (
            "/blog/a/,/blog/b/,308,exact",
            "has a status other than 301, 302, 307, 410 or 451",
        ),
        (
            "/blog/a/,/blog/b/,301,glob",
            "has a match other than exact or regex",
        ),
        (
            "/blog/a/,,301,exact",
            "moves to nowhere: a 301, 302 or 307 needs a to address",
        ),
        (
            "/blog/a/,/blog/b/,410,exact",
            "has a to address, which a 410 or 451 cannot use",
        ),
        ("/blog/a/,/blog/b/", "has fewer than three columns"),
        (
            "/blog/a/,/blog/b/,301,exact,more",
            "has more than four columns",
        ),
        ("\"/blog/a/,/blog/b/,301", "opens a quote it never closes"),
    ];
    for (row, problem) in cases {
        let text = format!("from,to,status,match\n/blog/fine/,/blog/b/,301\n{row}\n");
        let error = read_redirects_csv(&text).unwrap_err();
        assert_eq!(
            error,
            SeoError::RedirectCsvRow { line: 3, problem },
            "row {row}"
        );
    }
}

#[test]
fn a_bad_path_address_or_pattern_names_its_line_and_keeps_the_cause() {
    let cases = [
        "blog/a/,/blog/b/,301,exact",
        "/blog/a/,ftp://example.com/,301,exact",
        "/blog/a/,/blog/b c/,301,exact",
        "/blog/(/,/blog/b/,301,regex",
    ];
    for row in cases {
        let text = format!("from,to,status,match\n{row}\n");
        let error = read_redirects_csv(&text).unwrap_err();
        let SeoError::RedirectCsvValue { line, .. } = &error else {
            panic!("{row}: {error:?}");
        };
        assert_eq!(*line, 2, "row {row}");
        assert!(std::error::Error::source(&error).is_some(), "row {row}");
    }
}
