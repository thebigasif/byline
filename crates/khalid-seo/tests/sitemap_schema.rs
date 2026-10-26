//! The sitemap schema checks refuse what `sitemap.xsd` refuses (task 1.13).

#![cfg(test)]

mod sitemap_xsd;

use sitemap_xsd::{SITEMAP_NS, check_urlset, google_extensions, no_extension};

fn wrap(body: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"{SITEMAP_NS}\">{body}</urlset>"
    )
}

#[test]
fn a_minimal_valid_urlset_passes() {
    let good = wrap("<url><loc>https://example.com/</loc><lastmod>2026-10-02</lastmod></url>");
    assert_eq!(check_urlset(&good, no_extension), Ok(1));
    let alternate = wrap(
        "<url><loc>https://example.com/</loc>\
         <link xmlns=\"http://www.w3.org/1999/xhtml\" rel=\"alternate\" hreflang=\"de\" href=\"https://example.com/de/\"/>\
         </url>",
    );
    assert_eq!(check_urlset(&alternate, no_extension), Ok(1));
}

#[test]
fn every_rule_of_the_schema_refuses_its_violation() {
    for bad in [
        "<url><lastmod>2026-10-02</lastmod></url>",
        "<url><lastmod>2026-10-02</lastmod><loc>https://example.com/</loc></url>",
        "<url><loc>/relative/</loc></url>",
        "<url><loc>https://example.com/ü</loc></url>",
        "<url><loc>https://example.com/</loc><lastmod>yesterday</lastmod></url>",
        "<url><loc>https://example.com/</loc><priority>2</priority></url>",
        "<url><loc>https://example.com/</loc><changefreq>often</changefreq></url>",
        "<url><loc>https://example.com/</loc><extra/></url>",
        "<page><loc>https://example.com/</loc></page>",
    ] {
        assert!(check_urlset(&wrap(bad), no_extension).is_err(), "{bad}");
    }
}

#[test]
fn the_image_and_video_rules_refuse_a_broken_extension() {
    let wrap = |body: &str| {
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"{SITEMAP_NS}\" \
             xmlns:image=\"http://www.google.com/schemas/sitemap-image/1.1\" \
             xmlns:video=\"http://www.google.com/schemas/sitemap-video/1.1\">\
             <url><loc>https://example.com/</loc>{body}</url></urlset>"
        )
    };
    let video = "<video:thumbnail_loc>https://example.com/t.jpg</video:thumbnail_loc>\
                 <video:title>Links</video:title><video:description>Links</video:description>";
    for bad in [
        "<image:image></image:image>".to_owned(),
        "<video:video><video:title>Links</video:title></video:video>".to_owned(),
        format!("<video:video>{video}</video:video>"),
        format!(
            "<video:video>{video}<video:player_loc>https://example.com/p</video:player_loc>\
             <video:duration>0</video:duration></video:video>"
        ),
    ] {
        assert!(
            check_urlset(&wrap(&bad), google_extensions).is_err(),
            "{bad}"
        );
    }
}

#[test]
fn a_document_without_the_declaration_or_the_namespace_is_refused() {
    assert!(check_urlset("<urlset/>", no_extension).is_err());
    let no_namespace = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset></urlset>";
    assert!(check_urlset(no_namespace, no_extension).is_err());
}
