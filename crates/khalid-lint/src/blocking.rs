//! The five rules that block publishing by default (plan section 6).

use crate::{HeadingLevel, Hit, LinkKind, LintPost, Rule, RuleId, Severity, SiteView};

/// Whether a header field is missing or holds only spaces.
fn is_blank(field: Option<&str>) -> bool {
    field.is_none_or(|text| text.trim().is_empty())
}

/// A post with no publish date.
#[derive(Debug)]
pub(crate) struct NoDate;

impl Rule for NoDate {
    fn id(&self) -> RuleId {
        RuleId::new("no-date")
    }

    fn default_severity(&self) -> Severity {
        Severity::Block
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        post.date.is_none().then(Hit::post).into_iter().collect()
    }
}

/// A post with no named author.
#[derive(Debug)]
pub(crate) struct NoAuthor;

impl Rule for NoAuthor {
    fn id(&self) -> RuleId {
        RuleId::new("no-author")
    }

    fn default_severity(&self) -> Severity {
        Severity::Block
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        let blank = is_blank(post.author.as_deref());
        blank.then(Hit::post).into_iter().collect()
    }
}

/// A post that links to no other page of the site.
#[derive(Debug)]
pub(crate) struct NoInternalLink;

impl Rule for NoInternalLink {
    fn id(&self) -> RuleId {
        RuleId::new("no-internal-link")
    }

    fn default_severity(&self) -> Severity {
        Severity::Block
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        let internal = post
            .links
            .iter()
            .any(|link| link.kind == LinkKind::Internal);
        (!internal).then(Hit::post).into_iter().collect()
    }
}

/// A post with no title, which is the page's main headline.
#[derive(Debug)]
pub(crate) struct NoTitle;

impl Rule for NoTitle {
    fn id(&self) -> RuleId {
        RuleId::new("no-title")
    }

    fn default_severity(&self) -> Severity {
        Severity::Block
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        let blank = is_blank(post.title.as_deref());
        blank.then(Hit::post).into_iter().collect()
    }
}

/// A page with more than one main headline: the title counts as one, and
/// each `#` headline in the body as another.
#[derive(Debug)]
pub(crate) struct ManyHeadlines;

impl Rule for ManyHeadlines {
    fn id(&self) -> RuleId {
        RuleId::new("many-headlines")
    }

    fn default_severity(&self) -> Severity {
        Severity::Block
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        let body = post
            .headings
            .iter()
            .filter(|heading| heading.level == HeadingLevel::H1);
        // Without a title, the first body headline is the one allowed.
        let allowed = usize::from(is_blank(post.title.as_deref()));
        body.skip(allowed)
            .map(|heading| Hit::about(heading.text.clone()))
            .collect()
    }
}
