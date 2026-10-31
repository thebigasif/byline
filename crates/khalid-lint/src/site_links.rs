//! Site rules about how posts link to each other.

use crate::link_graph::LinkGraph;
use crate::{Hit, LintPost, RuleId, Severity, SiteRule, SiteView};

/// A post no other post links to. A language's only post is left out,
/// because no post could link to it yet.
#[derive(Debug)]
pub(crate) struct OrphanPost;

impl SiteRule for OrphanPost {
    fn id(&self) -> RuleId {
        RuleId::new("orphan-post")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check<'site>(&self, site: &'site dyn SiteView) -> Vec<(&'site LintPost, Hit)> {
        LinkGraph::new(site)
            .posts_with_sources()
            .filter(|(_, sources)| sources.is_empty())
            .filter(|(post, _)| {
                let mut in_language = site
                    .posts()
                    .iter()
                    .filter(|other| other.locale == post.locale);
                in_language.nth(1).is_some()
            })
            .map(|(post, _)| (post, Hit::post()))
            .collect()
    }
}

/// A post more clicks from the home page than the site allows, or one no
/// path from the home page reaches. Skipped when the home links are unknown.
#[derive(Debug)]
pub(crate) struct TooDeep {
    /// The most clicks allowed, from `Thresholds::max_clicks`.
    pub(crate) max: usize,
}

impl SiteRule for TooDeep {
    fn id(&self) -> RuleId {
        RuleId::new("too-deep")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check<'site>(&self, site: &'site dyn SiteView) -> Vec<(&'site LintPost, Hit)> {
        if site.home_links().is_empty() {
            return Vec::new();
        }
        LinkGraph::new(site)
            .posts_with_depth(site.home_links())
            .filter(|(_, depth)| depth.is_none_or(|clicks| clicks > self.max))
            .map(|(post, _)| (post, Hit::post()))
            .collect()
    }
}

/// A post exactly one other post links to, named in the finding.
#[derive(Debug)]
pub(crate) struct OneInboundLink;

impl SiteRule for OneInboundLink {
    fn id(&self) -> RuleId {
        RuleId::new("one-inbound-link")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check<'site>(&self, site: &'site dyn SiteView) -> Vec<(&'site LintPost, Hit)> {
        LinkGraph::new(site)
            .posts_with_sources()
            .filter_map(|(post, sources)| match sources.as_slice() {
                [only] => Some((post, Hit::about(only.slug.as_str()))),
                _ => None,
            })
            .collect()
    }
}
