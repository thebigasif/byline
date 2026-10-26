//! Site rules about how posts link to each other.

use crate::link_graph::LinkGraph;
use crate::{Hit, LintPost, RuleId, Severity, SiteRule, SiteView};

/// A post no other post links to.
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
            .map(|(post, _)| (post, Hit::post()))
            .collect()
    }
}
