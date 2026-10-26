//! Rules about a post's shape: headings, images, length and sources.

use crate::{HeadingLevel, Hit, LinkKind, LintPost, Rule, RuleId, Severity, SiteView};

/// A heading more than one level deeper than the heading before it. The
/// title is the `#` the first heading follows.
#[derive(Debug)]
pub(crate) struct HeadingOrder;

impl Rule for HeadingOrder {
    fn id(&self) -> RuleId {
        RuleId::new("heading-order")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        let mut previous = HeadingLevel::H1.number();
        let mut hits = Vec::new();
        for heading in &post.headings {
            let level = heading.level.number();
            if level > previous + 1 {
                hits.push(Hit::about(heading.text.clone()));
            }
            previous = level;
        }
        hits
    }
}

/// An image with no description, or one made only of spaces.
#[derive(Debug)]
pub(crate) struct MissingAlt;

impl Rule for MissingAlt {
    fn id(&self) -> RuleId {
        RuleId::new("missing-alt")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        post.images
            .iter()
            .filter(|image| image.alt.as_deref().is_none_or(|alt| alt.trim().is_empty()))
            .map(|image| Hit::about(image.src.clone()))
            .collect()
    }
}

/// A post with fewer words than the site's minimum.
#[derive(Debug)]
pub(crate) struct ThinPost {
    /// The fewest words a post may have, from `Thresholds::thin_post_words`.
    pub(crate) min_words: usize,
}

impl Rule for ThinPost {
    fn id(&self) -> RuleId {
        RuleId::new("thin-post")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        let thin = post.word_count < self.min_words;
        thin.then(Hit::post).into_iter().collect()
    }
}

/// A post that links to no other website.
#[derive(Debug)]
pub(crate) struct NoOutboundLink;

impl Rule for NoOutboundLink {
    fn id(&self) -> RuleId {
        RuleId::new("no-outbound-link")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        let cites = post
            .links
            .iter()
            .any(|link| link.kind == LinkKind::External);
        (!cites).then(Hit::post).into_iter().collect()
    }
}
