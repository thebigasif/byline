//! The checks on the files a crawler reads first: the sitemaps and what
//! they list, `robots.txt` and `llms.txt`, and the pages only a sitemap
//! leads to.

use crate::link_checks::inbound;
use crate::page_checks::robots;
use crate::{Check, Crawl, Issue, Outcome};

/// Runs the sitemap, `robots.txt` and `llms.txt` checks.
pub(crate) fn sitemap_issues(crawl: &Crawl, issues: &mut Vec<Issue>) {
    let files = crawl.files();
    let home = crawl.home();
    if files.robots().is_none() {
        issues.push(Issue::new(Check::RobotsMissing, home, ""));
    }
    match files.llms() {
        None => issues.push(Issue::new(Check::LlmsMissing, home, "")),
        Some(text) if !text.trim_start().starts_with("# ") => {
            issues.push(Issue::new(Check::LlmsFormat, home, ""));
        }
        Some(_) => {}
    }
    for sitemap in files.sitemaps().iter().filter(|map| map.status() != 200) {
        let check = if files.named_in_robots() {
            Check::SitemapBroken
        } else {
            Check::SitemapMissing
        };
        issues.push(Issue::new(
            check,
            sitemap.url(),
            sitemap.status().to_string(),
        ));
    }
    let inbound = inbound(crawl);
    for url in files.listed() {
        let Some(visit) = crawl.visit(url.as_str()) else {
            continue;
        };
        let check = match (visit.outcome(), visit.page()) {
            (Outcome::Redirect { .. }, _) => Check::SitemapRedirect,
            (outcome, _) if outcome.is_broken() => Check::SitemapBrokenPage,
            (_, Some(page)) if robots(visit, page).noindex() => Check::SitemapNoindex,
            (_, Some(page)) if page.canonical().is_some_and(|to| to != url) => {
                Check::SitemapNonCanonical
            }
            (_, Some(_)) if url != home && !inbound.contains_key(url.as_str()) => Check::Orphan,
            _ => continue,
        };
        issues.push(Issue::new(check, url, ""));
    }
}
