//! Finding a home page's stylesheets: page order, `<base>`, and the links
//! theme sync does not follow. Nothing here touches the network.

use khalid_core::AbsoluteUrl;
use khalid_theme::{PageStyle, SkipReason, Skipped, page_styles};

#[cfg(test)]
fn url(text: &str) -> AbsoluteUrl {
    AbsoluteUrl::parse(text).unwrap()
}

#[test]
fn lists_same_site_links_and_style_elements_in_page_order() {
    let html = r#"<!doctype html><html><head>
<link rel="stylesheet" href="/css/site.css">
<style>body { color: #1f2328; }</style>
<link rel="preload Stylesheet" href="https://assets.acme.example/theme.css">
<link rel="icon" href="/favicon.ico">
</head><body><h1>Acme Analytics</h1></body></html>"#;

    let styles = page_styles(html, &url("https://www.acme.example/"));

    assert_eq!(
        styles,
        [
            PageStyle::Linked(url("https://www.acme.example/css/site.css")),
            PageStyle::Inline("body { color: #1f2328; }".to_owned()),
            PageStyle::Linked(url("https://assets.acme.example/theme.css")),
        ]
    );
}

#[test]
fn a_base_element_moves_where_relative_links_point() {
    let html = r#"<head><base href="/en/"><link rel="stylesheet" href="main.css"></head>"#;

    let styles = page_styles(html, &url("https://acme.example/"));

    assert_eq!(
        styles,
        [PageStyle::Linked(url("https://acme.example/en/main.css"))]
    );
}

#[test]
fn a_link_to_another_site_is_skipped() {
    let html = r#"<link rel="stylesheet" href="https://cdn.tracker.example/t.css">"#;

    let styles = page_styles(html, &url("https://acme.example/"));

    assert_eq!(
        styles,
        [PageStyle::Skipped(Skipped::new(
            "https://cdn.tracker.example/t.css",
            SkipReason::OtherSite
        ))]
    );
}

#[test]
fn a_link_that_is_not_a_web_address_is_skipped() {
    let html = r#"<link rel="stylesheet" href="data:text/css,body{}">"#;

    let styles = page_styles(html, &url("https://acme.example/"));

    assert_eq!(
        styles,
        [PageStyle::Skipped(Skipped::new(
            "data:text/css,body{}",
            SkipReason::NotWebAddress
        ))]
    );
}

#[test]
fn print_dark_and_alternate_stylesheets_are_skipped_and_screen_ones_are_read() {
    let html = r#"<head>
<link rel="stylesheet" href="/print.css" media="print">
<link rel="stylesheet" href="/dark.css" media="(prefers-color-scheme: dark)">
<link rel="alternate stylesheet" href="/contrast.css" title="High contrast">
<link rel="stylesheet" href="/wide.css" media="screen and (min-width: 48em)">
<link rel="stylesheet" href="/both.css" media="print, screen">
</head>"#;

    let styles = page_styles(html, &url("https://acme.example/"));

    let not_for_screen =
        |href: &str| PageStyle::Skipped(Skipped::new(href, SkipReason::NotForScreen));
    assert_eq!(
        styles,
        [
            not_for_screen("https://acme.example/print.css"),
            not_for_screen("https://acme.example/dark.css"),
            not_for_screen("https://acme.example/contrast.css"),
            PageStyle::Linked(url("https://acme.example/wide.css")),
            PageStyle::Linked(url("https://acme.example/both.css")),
        ]
    );
}

#[test]
fn a_page_with_no_styles_lists_nothing() {
    let html = "<!doctype html><title>Loading</title><div id=root></div>";

    assert_eq!(page_styles(html, &url("https://acme.example/")), []);
}
