//! The checks each page answers on its own, from its body: text, links,
//! images, social tags and structured data.

use khalid_core::AbsoluteUrl;

use crate::page_checks::{Found, robots};
use crate::{Check, Crawl, Page, Thresholds, Visit};

/// Fewer words than this is a thin page.
const FEWEST_WORDS: usize = 200;
/// Text below this share of the HTML, in percent, is a low ratio.
const LOWEST_TEXT_PERCENT: usize = 10;
/// HTML above this many bytes is more than search engines read in full.
const MOST_HTML_BYTES: usize = 2 * 1024 * 1024;
/// The Open Graph tags a shared link needs.
const OPEN_GRAPH: [&str; 4] = ["og:title", "og:type", "og:image", "og:url"];

/// Runs the body checks on one page.
pub(crate) fn body_issues(
    crawl: &Crawl,
    (visit, page): (&Visit, &Page),
    thresholds: Thresholds,
    found: &mut Found<'_>,
) {
    text(visit, page, found);
    links(crawl, page, thresholds, found);
    extras(page, found);
}

/// The amount of text, against the page's size.
fn text(visit: &Visit, page: &Page, found: &mut Found<'_>) {
    if !robots(visit, page).noindex() && page.words() < FEWEST_WORDS {
        found.add(Check::LowWordCount, format!("{} words", page.words()));
    }
    let bytes = visit.bytes();
    let percent = page
        .text_bytes()
        .saturating_mul(100)
        .checked_div(bytes)
        .unwrap_or(100);
    if percent < LOWEST_TEXT_PERCENT {
        found.add(Check::LowTextRatio, format!("{percent}%"));
    }
    if bytes > MOST_HTML_BYTES {
        found.add(Check::HtmlTooLarge, format!("{bytes} bytes"));
    }
}

/// The page's links, its images and the files it loads.
fn links(crawl: &Crawl, page: &Page, thresholds: Thresholds, found: &mut Found<'_>) {
    let nofollow = page
        .links()
        .iter()
        .find(|link| link.nofollow() && crawl.is_inside(link.to()));
    if let Some(link) = nofollow {
        found.add(Check::NofollowInternal, link.to().as_str());
    }
    if page.links().len() > thresholds.most_links {
        found.add(Check::TooManyLinks, format!("{} links", page.links().len()));
    }
    if let Some(image) = page.images().iter().find(|image| image.alt().is_none()) {
        found.add(Check::AltMissing, image.src().as_str());
    }
    if !found.url.as_str().starts_with("https://") {
        return;
    }
    let plain = |url: &AbsoluteUrl| url.as_str().starts_with("http://");
    if let Some(link) = page.links().iter().find(|link| plain(link.to())) {
        found.add(Check::HttpLink, link.to().as_str());
    }
    if let Some(file) = page.resources().iter().find(|file| plain(file)) {
        found.add(Check::MixedContent, file.as_str());
    }
}

/// Social tags and structured data.
fn extras(page: &Page, found: &mut Found<'_>) {
    let missing: Vec<&str> = OPEN_GRAPH
        .into_iter()
        .filter(|tag| page.social().get(*tag).is_none_or(String::is_empty))
        .collect();
    if !missing.is_empty() {
        found.add(Check::OpenGraphIncomplete, missing.join(", "));
    }
    if !page.social().contains_key("twitter:card") {
        found.add(Check::TwitterCardMissing, "");
    }
    let blocks = page.structured_data();
    if blocks.is_empty() {
        found.add(Check::StructuredDataMissing, "");
    }
    let invalid = blocks
        .iter()
        .position(|block| serde_json::from_str::<serde::de::IgnoredAny>(block).is_err());
    if let Some(at) = invalid {
        found.add(
            Check::StructuredDataInvalid,
            format!("block {}", at.saturating_add(1)),
        );
    }
}
