//! Rules about the words a reader clicks and where they lead.

use std::collections::BTreeSet;

use crate::{Hit, LinkKind, LintLink, LintPost, Rule, RuleId, Severity, SiteView, german};

/// Link words that say nothing about the page they open, by language.
const VAGUE: &[(&str, &[&str])] = &[
    (
        "en",
        &[
            "click here",
            "here",
            "link",
            "more",
            "read more",
            "learn more",
            "this",
            "this post",
            "this article",
            "this page",
            "continue reading",
        ],
    ),
    ("de", german::VAGUE_LINK_TEXT),
];

/// Link words compared the way a reader sees them: lowercase, single spaces,
/// no full stop or colon at the end.
fn words(link: &LintLink) -> String {
    let lower = link.text.to_lowercase();
    let joined = lower.split_whitespace().collect::<Vec<_>>().join(" ");
    joined.trim_end_matches(['.', ':', '!', '…']).to_owned()
}

/// A link target without its `#section` and trailing slash, so two links to
/// the same page compare equal.
fn page(link: &LintLink) -> &str {
    let without_section = link.target.split('#').next().unwrap_or_default();
    without_section.trim_end_matches('/')
}

/// Every internal link on the site, in post order.
fn internal_links(site: &dyn SiteView) -> impl Iterator<Item = &LintLink> {
    site.posts()
        .iter()
        .flat_map(|post| &post.links)
        .filter(|link| link.kind == LinkKind::Internal)
}

/// One hit per distinct link words in `post` for which `flagged` holds.
fn per_distinct_words(post: &LintPost, flagged: impl Fn(&LintLink, &str) -> bool) -> Vec<Hit> {
    let mut seen = BTreeSet::new();
    post.links
        .iter()
        .filter(|link| link.kind == LinkKind::Internal)
        .filter_map(|link| {
            let text = words(link);
            let first = !text.is_empty() && seen.insert(text.clone());
            (first && flagged(link, &text)).then(|| Hit::about(link.text.trim()))
        })
        .collect()
}

/// The same link words used for a different page somewhere on the site.
#[derive(Debug)]
pub(crate) struct ReusedLinkText;

impl Rule for ReusedLinkText {
    fn id(&self) -> RuleId {
        RuleId::new("reused-link-text")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, site: &dyn SiteView) -> Vec<Hit> {
        per_distinct_words(post, |link, text| {
            internal_links(site).any(|other| words(other) == text && page(other) != page(link))
        })
    }
}

/// The same words linking to the same page too many times across the site.
#[derive(Debug)]
pub(crate) struct RepeatedLinkText {
    /// The most links allowed, from `Thresholds::same_link_text_max`.
    pub(crate) max: usize,
}

impl Rule for RepeatedLinkText {
    fn id(&self) -> RuleId {
        RuleId::new("repeated-link-text")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, site: &dyn SiteView) -> Vec<Hit> {
        per_distinct_words(post, |link, text| {
            let same = internal_links(site)
                .filter(|other| words(other) == text && page(other) == page(link));
            same.count() > self.max
        })
    }
}

/// Link words such as "click here" that do not name the page.
#[derive(Debug)]
pub(crate) struct VagueLinkText;

impl Rule for VagueLinkText {
    fn id(&self) -> RuleId {
        RuleId::new("vague-link-text")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        let language = post.locale.language();
        let vague = VAGUE
            .iter()
            .filter(|(lang, _)| *lang == language)
            .flat_map(|(_, phrases)| phrases.iter());
        let vague: BTreeSet<&str> = vague.copied().collect();
        post.links
            .iter()
            .filter(|link| vague.contains(words(link).as_str()))
            .map(|link| Hit::about(link.text.trim()))
            .collect()
    }
}

/// A second link to a page the post already links to.
#[derive(Debug)]
pub(crate) struct SameTargetTwice;

impl Rule for SameTargetTwice {
    fn id(&self) -> RuleId {
        RuleId::new("same-target-twice")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        let mut seen = BTreeSet::new();
        post.links
            .iter()
            .filter(|link| !seen.insert(page(link)))
            .map(|link| Hit::about(link.target.clone()))
            .collect()
    }
}

/// A link with no words, such as a linked image with no description.
#[derive(Debug)]
pub(crate) struct EmptyLinkText;

impl Rule for EmptyLinkText {
    fn id(&self) -> RuleId {
        RuleId::new("empty-link-text")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        post.links
            .iter()
            .filter(|link| link.text.trim().is_empty())
            .map(|link| Hit::about(link.target.clone()))
            .collect()
    }
}
