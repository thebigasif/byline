//! Reading a `WordPress` export file (WXR) into the export model.

use std::fs;

use chrono::NaiveDate;
use khalid_core::AbsoluteUrl;
use khalid_import::wordpress::{ItemKind, ItemStatus, WxrError, read_wxr};

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
        matches!(&error, WxrError::BadValue { field, line: 9, .. } if field == "link"),
        "{error:?}"
    );
}

#[test]
fn reads_every_item_in_file_order() {
    let export = read_wxr(&fixture("basic.xml")).unwrap();
    let items: Vec<(u64, &ItemKind, &ItemStatus)> = export
        .items
        .iter()
        .map(|item| (item.id, &item.kind, &item.status))
        .collect();
    let menu = ItemKind::Other("nav_menu_item".to_owned());
    assert_eq!(
        items,
        [
            (10, &ItemKind::Post, &ItemStatus::Publish),
            (11, &ItemKind::Page, &ItemStatus::Publish),
            (12, &ItemKind::Attachment, &ItemStatus::Inherit),
            (13, &ItemKind::Post, &ItemStatus::Draft),
            (14, &menu, &ItemStatus::Publish),
        ]
    );
}

#[test]
fn reads_a_post_with_its_terms_and_meta() {
    let export = read_wxr(&fixture("basic.xml")).unwrap();
    let post = &export.items[0];
    assert_eq!(post.title, "How to keep a starter alive");
    assert_eq!(post.name, "keep-a-starter-alive");
    assert_eq!(
        post.link.as_ref().map(AbsoluteUrl::as_str),
        Some("https://northfield.example.com/2024/03/keep-a-starter-alive/")
    );
    assert_eq!(post.creator, "mara");
    let local = NaiveDate::from_ymd_opt(2024, 3, 12).and_then(|day| day.and_hms_opt(9, 30, 0));
    assert_eq!(post.published, local, "the site's own time, not GMT");
    assert!(post.content.contains("<strong>every day</strong>"));
    assert_eq!(post.excerpt, "A starter needs food, warmth and time.");
    assert!(post.sticky);
    assert_eq!(post.categories, ["sourdough"]);
    assert_eq!(post.tags, ["rye"]);
    assert_eq!(post.meta("_thumbnail_id"), Some("12"));
    assert_eq!(post.parent, None);
    assert!(!export.items[1].sticky);
}

#[test]
fn reads_an_attachment_and_a_draft() {
    let export = read_wxr(&fixture("basic.xml")).unwrap();
    let image = export.item(12).unwrap();
    assert_eq!(image.parent, Some(10));
    assert_eq!(
        image.attachment_url.as_ref().map(AbsoluteUrl::as_str),
        Some("https://northfield.example.com/wp-content/uploads/2024/03/starter-jar.jpg")
    );
    assert_eq!(
        image.meta("_wp_attachment_image_alt"),
        Some("A bubbling starter in a glass jar")
    );
    let draft = export.item(13).unwrap();
    assert_eq!(draft.published, None, "WordPress writes zeros for no date");
    assert_eq!(draft.name, "");
    assert_eq!(draft.categories, ["baking"]);
    assert!(export.item(99).is_none());
}

#[test]
fn names_the_line_of_a_bad_post_id() {
    let text = fixture("basic.xml").replace(
        "<wp:post_id>11</wp:post_id>",
        "<wp:post_id>eleven</wp:post_id>",
    );
    let error = read_wxr(&text).unwrap_err();
    let WxrError::BadValue { line, field, value } = &error else {
        panic!("{error:?}");
    };
    assert_eq!((field.as_str(), value.as_str()), ("wp:post_id", "eleven"));
    let expected = text.lines().position(|row| row.contains("eleven")).unwrap() + 1;
    assert_eq!(*line, expected);
}

#[test]
fn names_the_field_of_a_bad_date() {
    let text = fixture("basic.xml").replace(
        "2023-01-05 12:00:00]]></wp:post_date>",
        "yesterday]]></wp:post_date>",
    );
    let error = read_wxr(&text).unwrap_err();
    assert!(
        matches!(&error, WxrError::BadValue { field, value, .. } if field == "wp:post_date" && value == "yesterday"),
        "{error:?}"
    );
}
