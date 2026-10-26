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
pub(crate) fn check_loc(node: Node<'_, '_>) -> Result<(), String> {
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
pub(crate) fn check_lastmod(node: Node<'_, '_>) -> Result<(), String> {
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

const IMAGE_NS: &str = "http://www.google.com/schemas/sitemap-image/1.1";
const VIDEO_NS: &str = "http://www.google.com/schemas/sitemap-video/1.1";
const NEWS_NS: &str = "http://www.google.com/schemas/sitemap-news/0.9";

fn children<'a, 'i>(node: Node<'a, 'i>, ns: &str) -> Vec<(&'a str, Node<'a, 'i>)> {
    node.children()
        .filter(|child| child.is_element() && child.tag_name().namespace() == Some(ns))
        .map(|child| (child.tag_name().name(), child))
        .collect()
}

fn text_of<'a>(found: &[(&str, Node<'a, '_>)], name: &str) -> Option<&'a str> {
    found
        .iter()
        .find(|(n, _)| *n == name)
        .and_then(|(_, node)| node.text())
}

/// `sitemap-image` 1.1: `image:image` holds exactly one `image:loc`.
fn check_image(node: Node<'_, '_>) -> Result<(), String> {
    let found = children(node, IMAGE_NS);
    match found.as_slice() {
        [("loc", loc)] => check_loc(*loc),
        _ => Err("image:image must hold exactly one image:loc".to_owned()),
    }
}

/// `sitemap-video` 1.1: thumbnail, title, description of at most 2048
/// characters, a content or player address, a duration of 1 to 28,800
/// seconds, and a W3C publication date.
fn check_video(node: Node<'_, '_>) -> Result<(), String> {
    let found = children(node, VIDEO_NS);
    for required in ["thumbnail_loc", "title", "description"] {
        if text_of(&found, required).is_none_or(str::is_empty) {
            return Err(format!("video:video has no {required}"));
        }
    }
    if text_of(&found, "description").is_some_and(|d| d.chars().count() > 2048) {
        return Err("video:description is over 2048 characters".to_owned());
    }
    if text_of(&found, "content_loc").is_none() && text_of(&found, "player_loc").is_none() {
        return Err("video:video needs content_loc or player_loc".to_owned());
    }
    if let Some(duration) = text_of(&found, "duration") {
        let seconds: u32 = duration
            .parse()
            .map_err(|_| "video:duration is not whole seconds")?;
        if !(1..=28_800).contains(&seconds) {
            return Err(format!("video:duration {seconds} is outside 1 to 28800"));
        }
    }
    if let Some((_, date)) = found.iter().find(|(n, _)| *n == "publication_date") {
        check_lastmod(*date)?;
    }
    Ok(())
}

/// `sitemap-news` 0.9: a publication with a name and a language, a W3C
/// publication date and a title.
fn check_news(node: Node<'_, '_>) -> Result<(), String> {
    let found = children(node, NEWS_NS);
    let publication = found
        .iter()
        .find(|(n, _)| *n == "publication")
        .map(|(_, p)| children(*p, NEWS_NS))
        .ok_or("news:news has no publication")?;
    if text_of(&publication, "name").is_none_or(str::is_empty)
        || text_of(&publication, "language").is_none_or(|l| l.len() < 2)
    {
        return Err("news:publication needs a name and a language".to_owned());
    }
    let date = found
        .iter()
        .find(|(n, _)| *n == "publication_date")
        .ok_or("news:news has no publication_date")?;
    check_lastmod(date.1)?;
    if text_of(&found, "title").is_none_or(str::is_empty) {
        return Err("news:news has no title".to_owned());
    }
    Ok(())
}

/// The Google image, video and news extensions, each against its schema.
pub(crate) fn google_extensions(node: Node<'_, '_>) -> Result<(), String> {
    match (node.tag_name().namespace(), node.tag_name().name()) {
        (Some(IMAGE_NS), "image") => check_image(node),
        (Some(VIDEO_NS), "video") => check_video(node),
        (Some(NEWS_NS), "news") => check_news(node),
        _ => no_extension(node),
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
