//! The checks that need the whole crawl: broken pages and links,
//! redirects, broken files, click depth and inbound links.

use std::collections::{BTreeMap, BTreeSet};

use khalid_core::AbsoluteUrl;

use crate::{Check, Crawl, Issue, Outcome, Page, Visit};

/// More clicks from home than this is too deep.
const MOST_CLICKS: u32 = 3;
/// A redirect chain longer than this is followed no further.
const MOST_HOPS: usize = 20;

/// Runs every whole-crawl check.
pub(crate) fn link_issues(crawl: &Crawl, issues: &mut Vec<Issue>) {
    let inbound = inbound(crawl);
    for visit in crawl.visits() {
        let url = visit.url();
        if let Some(check) = broken(visit.outcome())
            && (url == crawl.home() || inbound.contains_key(url.as_str()))
        {
            issues.push(Issue::new(check, url, status(visit.outcome())));
        }
        if let Outcome::Redirect { .. } = visit.outcome() {
            match hops(crawl, visit) {
                Hops::Loop => issues.push(Issue::new(Check::RedirectLoop, url, "")),
                Hops::End(count) if count > 1 => {
                    issues.push(Issue::new(
                        Check::RedirectChain,
                        url,
                        format!("{count} hops"),
                    ));
                }
                Hops::End(_) => {}
            }
        }
        let Some(page) = visit.page() else { continue };
        targets(crawl, visit, page, issues);
        if let Some(clicks) = visit.depth().filter(|clicks| *clicks > MOST_CLICKS) {
            issues.push(Issue::new(Check::TooDeep, url, format!("{clicks} clicks")));
        }
        let from = inbound.get(url.as_str()).map_or(0, BTreeSet::len);
        if url != crawl.home() && from == 1 {
            issues.push(Issue::new(Check::OneInboundLink, url, ""));
        }
    }
}

/// The check a broken outcome fails, if it is broken.
fn broken(outcome: &Outcome) -> Option<Check> {
    match outcome {
        Outcome::Status(400..=499) => Some(Check::Page4xx),
        Outcome::Status(500..) => Some(Check::Page5xx),
        Outcome::Unreached(_) => Some(Check::PageUnreached),
        _ => None,
    }
}

/// The status as a detail, such as `404`, or why there was none.
fn status(outcome: &Outcome) -> String {
    match outcome {
        Outcome::Status(code) | Outcome::Redirect { status: code, .. } => code.to_string(),
        Outcome::Unreached(why) => format!("{why:?}").to_lowercase(),
    }
}

/// The pages each address inside the blog is linked from, without
/// `nofollow` links and links from a page to itself.
fn inbound(crawl: &Crawl) -> BTreeMap<&str, BTreeSet<&str>> {
    let mut from: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for visit in crawl.visits() {
        let Some(page) = visit.page() else { continue };
        for link in page.links().iter().filter(|link| !link.nofollow()) {
            if link.to() != visit.url() && crawl.is_inside(link.to()) {
                from.entry(link.to().as_str())
                    .or_default()
                    .insert(visit.url().as_str());
            }
        }
    }
    from
}

/// Where a redirect chain ends.
enum Hops {
    /// After this many redirects, at an address that does not redirect.
    End(usize),
    /// Back at an address already passed.
    Loop,
}

/// Follows the redirects from `visit` through the crawl.
fn hops(crawl: &Crawl, visit: &Visit) -> Hops {
    let mut passed = BTreeSet::from([visit.url().as_str()]);
    let mut at = visit;
    for count in 1..=MOST_HOPS {
        let Outcome::Redirect { to, .. } = at.outcome() else {
            return Hops::End(count.saturating_sub(1));
        };
        if !passed.insert(to.as_str()) {
            return Hops::Loop;
        }
        match crawl.visit(to.as_str()) {
            Some(next) => at = next,
            None => return Hops::End(count),
        }
    }
    Hops::Loop
}

/// The page's links, images and files that are broken or redirect, once
/// each.
fn targets(crawl: &Crawl, visit: &Visit, page: &Page, issues: &mut Vec<Issue>) {
    let images: BTreeSet<&AbsoluteUrl> = page.images().iter().map(crate::Image::src).collect();
    let links = page.links().iter().map(|link| (link.to(), Kind::Link));
    let files = page.resources().iter().map(|file| {
        let kind = if images.contains(file) {
            Kind::Image
        } else {
            Kind::File
        };
        (file, kind)
    });
    let mut seen = BTreeSet::new();
    for (to, kind) in links.chain(files) {
        let Some(target) = crawl.visit(to.as_str()) else {
            continue;
        };
        if !seen.insert((to.as_str(), kind)) {
            continue;
        }
        let check = match (kind, target.outcome()) {
            (Kind::Link, Outcome::Redirect { .. }) => Check::LinksToRedirect,
            (_, outcome) if !outcome.is_broken() => continue,
            (Kind::Link, _) => Check::BrokenLink,
            (Kind::Image, _) => Check::ImageBroken,
            (Kind::File, _) => Check::ResourceBroken,
        };
        issues.push(Issue::new(check, visit.url(), to.as_str()));
    }
}

/// How a page refers to an address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    /// A link a reader clicks.
    Link,
    /// An image it shows.
    Image,
    /// A stylesheet or script it loads.
    File,
}
