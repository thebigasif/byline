//! The rules of the sitemap protocol's XML schemas, checked in tests.
//!
//! No XSD validator exists in pure Rust, so this module enforces what
//! `sitemap.xsd` 0.9 (sitemaps.org) and the Google image, video and news
//! extension schemas require, rule by rule, on a parsed document. Each
//! check names the rule it enforces.

#![cfg(test)]

use roxmltree::{Document, Node};

pub(crate) const SITEMAP_NS: &str = "http://www.sitemaps.org/schemas/sitemap/0.9";
const XHTML_NS: &str = "http://www.w3.org/1999/xhtml";

/// The protocol's limits: 50,000 URLs and 50 MiB uncompressed per file.
const MAX_URLS: usize = 50_000;
const MAX_BYTES: usize = 52_428_800;

/// `tLoc`: a URL between 12 and 2048 characters.
fn check_loc(node: Node<'_, '_>) -> Result<(), String> {
    let text = node.text().unwrap_or_default();
    if !(12..=2048).contains(&text.chars().count()) {
        return Err(format!("loc {text:?} is not 12 to 2048 characters"));
    }
    if !(text.starts_with("https://") || text.starts_with("http://")) {
        return Err(format!("loc {text:?} is not an absolute http(s) URL"));
    }
    if text.chars().any(|c| c.is_whitespace() || !c.is_ascii()) {
        return Err(format!("loc {text:?} is not percent-encoded"));
    }
    Ok(())
}

/// `tLastmod`: a W3C datetime, either a date or a full time with a zone.
fn check_lastmod(node: Node<'_, '_>) -> Result<(), String> {
    let text = node.text().unwrap_or_default();
    let date = chrono::NaiveDate::parse_from_str(text, "%Y-%m-%d").is_ok();
    let time = chrono::DateTime::parse_from_rfc3339(text).is_ok();
    if date || time {
        Ok(())
    } else {
        Err(format!("lastmod {text:?} is not a W3C datetime"))
    }
}

fn is_sitemap(node: Node<'_, '_>, name: &str) -> bool {
    node.is_element()
        && node.tag_name().name() == name
        && node.tag_name().namespace() == Some(SITEMAP_NS)
}

/// `xhtml:link` inside a `url`: `rel="alternate"`, `hreflang` and `href`.
fn check_xhtml_link(node: Node<'_, '_>) -> Result<(), String> {
    let ok = node.tag_name().name() == "link"
        && node.attribute("rel") == Some("alternate")
        && node.attribute("hreflang").is_some_and(|v| !v.is_empty())
        && node
            .attribute("href")
            .is_some_and(|v| v.starts_with("http"));
    if ok {
        Ok(())
    } else {
        Err("xhtml:link needs rel=alternate, hreflang and an absolute href".to_owned())
    }
}

/// One `url`: `loc`, then `lastmod`, `changefreq` and `priority` in that
/// order, each at most once, then elements from other namespaces only.
fn check_url(
    url: Node<'_, '_>,
    extension: fn(Node<'_, '_>) -> Result<(), String>,
) -> Result<(), String> {
    let order = ["loc", "lastmod", "changefreq", "priority"];
    let mut position = 0;
    let mut seen_extension = false;
    let mut has_loc = false;
    for child in url.children().filter(Node::is_element) {
        let ns = child.tag_name().namespace();
        if ns == Some(SITEMAP_NS) {
            let name = child.tag_name().name();
            let index = order
                .iter()
                .position(|n| *n == name)
                .ok_or_else(|| format!("unknown sitemap element {name}"))?;
            if seen_extension || index < position {
                return Err(format!("{name} is out of order"));
            }
            match name {
                "loc" => {
                    check_loc(child)?;
                    has_loc = true;
                }
                "lastmod" => check_lastmod(child)?,
                "changefreq" => {
                    let text = child.text().unwrap_or_default();
                    let allowed = [
                        "always", "hourly", "daily", "weekly", "monthly", "yearly", "never",
                    ];
                    if !allowed.contains(&text) {
                        return Err(format!("changefreq {text:?} is not allowed"));
                    }
                }
                _ => {
                    let value: f64 = child
                        .text()
                        .unwrap_or_default()
                        .parse()
                        .map_err(|_| "priority is not a number")?;
                    if !(0.0..=1.0).contains(&value) {
                        return Err(format!("priority {value} is outside 0.0 to 1.0"));
                    }
                }
            }
            position = index + 1;
        } else if ns.is_none() {
            return Err("an element with no namespace is not allowed in url".to_owned());
        } else {
            if !has_loc {
                return Err("an extension element comes before loc".to_owned());
            }
            seen_extension = true;
            if ns == Some(XHTML_NS) {
                check_xhtml_link(child)?;
            } else {
                extension(child)?;
            }
        }
    }
    if has_loc {
        Ok(())
    } else {
        Err("a url has no loc".to_owned())
    }
}

/// No extension namespaces beyond `xhtml:link` are expected.
pub(crate) fn no_extension(node: Node<'_, '_>) -> Result<(), String> {
    Err(format!(
        "unexpected extension element {:?}",
        node.tag_name()
    ))
}

/// Checks a `urlset` document against `sitemap.xsd` and the protocol limits.
pub(crate) fn check_urlset(
    xml: &str,
    extension: fn(Node<'_, '_>) -> Result<(), String>,
) -> Result<usize, String> {
    if !xml.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n") {
        return Err("the file does not start with the UTF-8 XML declaration".to_owned());
    }
    if xml.len() > MAX_BYTES {
        return Err("the file is over 50 MiB".to_owned());
    }
    let document = Document::parse(xml).map_err(|error| error.to_string())?;
    let root = document.root_element();
    if !is_sitemap(root, "urlset") {
        return Err("the root is not urlset in the sitemap namespace".to_owned());
    }
    let mut count = 0;
    for child in root.children().filter(Node::is_element) {
        if !is_sitemap(child, "url") {
            return Err(format!("urlset holds {:?}", child.tag_name()));
        }
        check_url(child, extension)?;
        count += 1;
    }
    if count > MAX_URLS {
        return Err(format!("{count} URLs is over the limit of 50,000"));
    }
    Ok(count)
}

#[test]
fn the_validator_refuses_what_the_schema_refuses() {
    let head = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";
    let wrap = |body: &str| format!("{head}<urlset xmlns=\"{SITEMAP_NS}\">{body}</urlset>");
    let good = wrap("<url><loc>https://example.com/</loc><lastmod>2026-10-02</lastmod></url>");
    assert_eq!(check_urlset(&good, no_extension), Ok(1));
    for bad in [
        "<url><lastmod>2026-10-02</lastmod></url>",
        "<url><lastmod>2026-10-02</lastmod><loc>https://example.com/</loc></url>",
        "<url><loc>/relative/</loc></url>",
        "<url><loc>https://example.com/</loc><lastmod>yesterday</lastmod></url>",
        "<url><loc>https://example.com/</loc><priority>2</priority></url>",
        "<url><loc>https://example.com/</loc><changefreq>often</changefreq></url>",
        "<url><loc>https://example.com/</loc><extra/></url>",
        "<page><loc>https://example.com/</loc></page>",
    ] {
        assert!(check_urlset(&wrap(bad), no_extension).is_err(), "{bad}");
    }
    assert!(check_urlset("<urlset/>", no_extension).is_err());
}
