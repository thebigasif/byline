//! Reading a `WordPress` export file (WXR) into the export model.

use std::fs;

use khalid_import::wordpress::{WxrError, read_wxr};

#[cfg(test)]
fn fixture(name: &str) -> String {
    fs::read_to_string(format!(
        "{}/fixtures/wordpress/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

#[test]
fn reads_the_site_authors_and_terms() {
    let export = read_wxr(&fixture("basic.xml")).unwrap();
    let site = &export.site;
    assert_eq!(site.title, "Northfield Bakery Blog");
    assert_eq!(site.address.as_str(), "https://northfield.example.com/");
    assert_eq!(site.language.as_deref(), Some("en-US"));

    let authors: Vec<[&str; 2]> = export
        .authors
        .iter()
        .map(|a| [a.login.as_str(), a.display_name.as_str()])
        .collect();
    assert_eq!(authors, [["mara", "Mara Lind"], ["tom.b", "Tom Becker"]]);

    let terms: Vec<String> = export
        .categories
        .iter()
        .chain(&export.tags)
        .map(|t| format!("{:?} {} {} {:?}", t.id, t.slug, t.name, t.parent))
        .collect();
    // WordPress stores term names with HTML entities; the model keeps them.
    assert_eq!(
        terms,
        [
            "Some(3) baking Baking None",
            "Some(4) sourdough Sourdough &amp; starters Some(\"baking\")",
            "Some(5) rye Rye None",
        ]
    );
}

#[test]
fn leaves_out_the_authors_email_addresses() {
    let export = read_wxr(&fixture("basic.xml")).unwrap();
    let model = format!("{export:?}");
    assert!(!model.contains('@'), "{model}");
}

#[test]
fn refuses_text_that_is_not_xml() {
    let error = read_wxr("<rss><channel>").unwrap_err();
    assert!(matches!(error, WxrError::NotXml { .. }), "{error:?}");
}

#[test]
fn refuses_a_feed_that_is_not_a_wordpress_export() {
    let feed = r#"<?xml version="1.0"?><rss version="2.0"><channel><title>News</title><link>https://example.com</link></channel></rss>"#;
    let error = read_wxr(feed).unwrap_err();
    assert!(matches!(error, WxrError::NotWordPress), "{error:?}");
}

#[test]
fn names_the_line_of_a_bad_term_id() {
    let text = fixture("basic.xml").replace(
        "<wp:term_id>4</wp:term_id>",
        "<wp:term_id>four</wp:term_id>",
    );
    let error = read_wxr(&text).unwrap_err();
    let WxrError::BadValue { line, field, value } = &error else {
        panic!("{error:?}");
    };
    assert_eq!((field.as_str(), value.as_str()), ("wp:term_id", "four"));
    let expected = text.lines().position(|row| row.contains(">four<")).unwrap() + 1;
    assert_eq!(*line, expected);
}

#[test]
fn names_the_line_of_a_bad_address() {
    let text = fixture("basic.xml").replace(
        "<link>https://northfield.example.com</link>",
        "<link>northfield bakery</link>",
    );
    let error = read_wxr(&text).unwrap_err();
    assert!(
        matches!(&error, WxrError::BadValue { field, line: 6, .. } if field == "link"),
        "{error:?}"
    );
}
