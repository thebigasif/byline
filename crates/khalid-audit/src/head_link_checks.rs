//! The checks on what a page's head points at, canonical and `hreflang`,
//! and on titles and descriptions two pages share.

use std::collections::BTreeMap;

use crate::page_checks::robots;
use crate::{Check, Crawl, Issue, Outcome, Page, Visit};

/// Runs the canonical, `hreflang` and duplicate checks.
pub(crate) fn head_link_issues(crawl: &Crawl, issues: &mut Vec<Issue>) {
    let mut titles: BTreeMap<&str, Vec<&Visit>> = BTreeMap::new();
    let mut descriptions: BTreeMap<&str, Vec<&Visit>> = BTreeMap::new();
    for visit in crawl.visits() {
        let Some(page) = visit.page() else { continue };
        canonical(crawl, visit, page, issues);
        alternates(crawl, visit, page, issues);
        if listed(visit, page) {
            if let Some(title) = page.titles().first().filter(|text| !text.is_empty()) {
                titles.entry(title).or_default().push(visit);
            }
            if let Some(text) = page.descriptions().first().filter(|text| !text.is_empty()) {
                descriptions.entry(text).or_default().push(visit);
            }
        }
    }
    let shared = [
        (titles, Check::TitleDuplicate),
        (descriptions, Check::DescriptionDuplicate),
    ];
    for (texts, check) in shared {
        for pages in texts.into_values().filter(|pages| pages.len() > 1) {
            for visit in &pages {
                let other = pages.iter().find(|other| other.url() != visit.url());
                let detail = other.map(|other| other.url().as_str()).unwrap_or_default();
                issues.push(Issue::new(check, visit.url(), detail));
            }
        }
    }
}

/// Whether search engines list `page` under its own address: not noindex,
/// and canonical to itself or to nothing.
fn listed(visit: &Visit, page: &Page) -> bool {
    !robots(visit, page).noindex() && page.canonical().is_none_or(|url| url == visit.url())
}

/// A canonical that points at a broken page or a redirect.
fn canonical(crawl: &Crawl, visit: &Visit, page: &Page, issues: &mut Vec<Issue>) {
    let Some(to) = page.canonical() else { return };
    let check = match crawl.visit(to.as_str()).map(Visit::outcome) {
        Some(Outcome::Redirect { .. }) => Check::CanonicalRedirect,
        Some(outcome) if outcome.is_broken() => Check::CanonicalBroken,
        _ => return,
    };
    issues.push(Issue::new(check, visit.url(), to.as_str()));
}

/// Translations that are broken, redirect, do not name this page back, or
/// leave this page out.
fn alternates(crawl: &Crawl, visit: &Visit, page: &Page, issues: &mut Vec<Issue>) {
    if page.alternates().is_empty() {
        return;
    }
    if !page
        .alternates()
        .iter()
        .any(|alternate| alternate.to() == visit.url())
    {
        issues.push(Issue::new(Check::HreflangNoSelf, visit.url(), ""));
    }
    for alternate in page.alternates() {
        let to = alternate.to();
        let Some(target) = crawl.visit(to.as_str()).filter(|_| to != visit.url()) else {
            continue;
        };
        let check = match (target.outcome(), target.page()) {
            (Outcome::Redirect { .. }, _) => Check::HreflangRedirect,
            (outcome, _) if outcome.is_broken() => Check::HreflangBroken,
            (_, Some(other))
                if !other
                    .alternates()
                    .iter()
                    .any(|back| back.to() == visit.url()) =>
            {
                Check::HreflangNoReturn
            }
            _ => continue,
        };
        issues.push(Issue::new(check, visit.url(), to.as_str()));
    }
}
