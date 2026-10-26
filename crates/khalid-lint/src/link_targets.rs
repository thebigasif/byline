//! Rules about where a post's links lead: old addresses and sister sites.

use crate::link_graph::page_key;
use crate::link_hygiene::{bare_host, web_host};
use crate::{Hit, LinkKind, LintPost, Rule, RuleId, Severity, SiteView};

/// An internal link to an old path of a post, listed in its `aliases`,
/// which now redirects. Suggests the post's current path when it is known.
/// Dead addresses and outside redirects need a fetch, so `khalid audit`
/// checks those.
#[derive(Debug)]
pub(crate) struct RedirectingLink;

impl Rule for RedirectingLink {
    fn id(&self) -> RuleId {
        RuleId::new("redirecting-link")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, site: &dyn SiteView) -> Vec<Hit> {
        post.links
            .iter()
            .filter(|link| link.kind == LinkKind::Internal)
            .filter_map(|link| {
                let page = page_key(&link.target);
                let moved = site
                    .posts()
                    .iter()
                    .find(|other| other.aliases.iter().any(|alias| page_key(alias) == page))?;
                let current = moved.path.iter().cloned().collect();
                Some(Hit::about(link.target.clone()).suggesting(current))
            })
            .collect()
    }
}

/// A link to a sister site served by the same Khalid process, over either
/// scheme and with or without `www.`.
#[derive(Debug)]
pub(crate) struct CrossSiteLink;

impl Rule for CrossSiteLink {
    fn id(&self) -> RuleId {
        RuleId::new("cross-site-link")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, site: &dyn SiteView) -> Vec<Hit> {
        let sisters: Vec<String> = site.sibling_hosts().iter().map(|h| bare_host(h)).collect();
        if sisters.is_empty() {
            return Vec::new();
        }
        post.links
            .iter()
            .filter(|link| {
                web_host(&link.target).is_some_and(|host| sisters.contains(&bare_host(host)))
            })
            .map(|link| Hit::about(link.target.clone()))
            .collect()
    }
}
