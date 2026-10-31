//! Rules about the words a reader clicks and where they lead.

use std::collections::BTreeSet;

use crate::link_graph::{opens, page_key};
use crate::{Hit, LinkKind, LintLink, LintPost, Rule, RuleId, Severity, SiteView, focus, german};

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

/// The page a link opens, so two links to the same page compare equal.
fn page(link: &LintLink) -> &str {
    page_key(&link.target)
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
        post.links
            .iter()
            .filter(|link| is_vague(language, link))
            .map(|link| Hit::about(link.text.trim()))
            .collect()
    }
}

/// Whether the words of `link` are vague in `language`.
fn is_vague(language: &str, link: &LintLink) -> bool {
    let text = words(link);
    VAGUE
        .iter()
        .filter(|(lang, _)| *lang == language)
        .any(|(_, phrases)| phrases.contains(&text.as_str()))
}

/// The fewest first letters two forms of a word share, such as "choose" and
/// "choosing", to count as the same word.
const SAME_WORD_LETTERS: usize = 4;

/// Whether `one` and `other` are the same word, or forms of it that share
/// their first letters.
fn same_word(one: &str, other: &str) -> bool {
    let shared = one
        .chars()
        .zip(other.chars())
        .take_while(|(a, b)| a == b)
        .count();
    one == other || shared >= SAME_WORD_LETTERS
}

/// The words of `text` that can name a subject in `language`, without
/// filler and question words.
fn subject_words(language: &str, text: &str) -> Vec<String> {
    let all = focus::words(text).into_iter();
    all.filter(|word| crate::words::names_subject(language, word))
        .collect()
}

/// The words that name what `post` is about: from its title, its search
/// title and its focus keyword.
fn named_by(post: &LintPost) -> Vec<String> {
    let language = post.locale.language();
    [&post.title, &post.seo_title, &post.focus_keyword]
        .into_iter()
        .flatten()
        .flat_map(|text| subject_words(language, text))
        .collect()
}

/// Link words that share no word with the title of the post they open.
///
/// Koray Tugberk Gubur: link words should carry the words of the target
/// page's title, because search engines read them as a label for it. Vague
/// and empty links are left to their own rules.
#[derive(Debug)]
pub(crate) struct LinkTextMissesTarget;

impl Rule for LinkTextMissesTarget {
    fn id(&self) -> RuleId {
        RuleId::new("link-text-misses-target")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, site: &dyn SiteView) -> Vec<Hit> {
        let language = post.locale.language();
        let internal = post.links.iter().filter(|l| l.kind == LinkKind::Internal);
        internal
            .filter(|link| !is_vague(language, link))
            .filter_map(|link| {
                let said = subject_words(language, &link.text);
                let target = site
                    .posts()
                    .iter()
                    .find(|to| opens(&post.locale, &link.target, to))?;
                let named = named_by(target);
                let misses = !said.is_empty()
                    && !named.is_empty()
                    && !said.iter().any(|w| named.iter().any(|n| same_word(w, n)));
                misses.then(|| Hit::about(link.text.trim()))
            })
            .collect()
    }
}

/// Link words to another page that hold the post's own focus keyword.
///
/// Koray Tugberk Gubur: a page should not link away with the words of its
/// own main topic, because the link words tell search engines that the
/// other page is the one to show for them.
#[derive(Debug)]
pub(crate) struct LinkTextIsOwnFocus;

impl Rule for LinkTextIsOwnFocus {
    fn id(&self) -> RuleId {
        RuleId::new("link-text-is-own-focus")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        let topic = post
            .focus_keyword
            .as_deref()
            .map(focus::words)
            .unwrap_or_default();
        post.links
            .iter()
            .filter(|link| link.kind == LinkKind::Internal)
            .filter(|link| !opens(&post.locale, &link.target, post))
            .filter(|link| focus::holds(&focus::words(&link.text), &topic))
            .map(|link| Hit::about(link.text.trim()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use khalid_core::{Locale, Slug};

    use super::*;
    use crate::LintSite;

    fn post(slug: &str) -> LintPost {
        LintPost::new(Locale::try_new("en").unwrap(), Slug::try_new(slug).unwrap())
    }

    #[test]
    fn another_form_of_a_title_word_matches_the_target() {
        let mut target = post("choosing-link-text");
        target.title = Some("How to choose link text".to_owned());
        let mut from = post("a");
        let link = LintLink::new(LinkKind::Internal, "/blog/choosing-link-text/", "");
        for (text, hits) in [("choosing your links", 0), ("a guide from spring", 1)] {
            from.links = vec![LintLink {
                text: text.to_owned(),
                ..link.clone()
            }];
            let site = LintSite {
                posts: vec![target.clone()],
                ..LintSite::default()
            };
            assert_eq!(
                LinkTextMissesTarget.check(&from, &site).len(),
                hits,
                "{text}"
            );
        }
    }

    #[test]
    fn vague_or_filler_words_and_unknown_targets_are_left_alone() {
        let mut target = post("choosing-link-text");
        target.title = Some("How to choose link text".to_owned());
        let mut from = post("a");
        from.links = vec![
            LintLink::new(
                LinkKind::Internal,
                "/blog/choosing-link-text/",
                "click here",
            ),
            LintLink::new(LinkKind::Internal, "/blog/choosing-link-text/", "how to"),
            LintLink::new(LinkKind::Internal, "/blog/elsewhere/", "spring guide"),
        ];
        let site = LintSite {
            posts: vec![target],
            ..LintSite::default()
        };
        assert_eq!(LinkTextMissesTarget.check(&from, &site), []);
    }

    #[test]
    fn the_own_focus_keyword_may_link_to_the_post_itself() {
        let mut from = post("internal-links");
        from.focus_keyword = Some("Internal links".to_owned());
        from.links = vec![
            LintLink::new(
                LinkKind::Internal,
                "/blog/internal-links/#why",
                "internal links",
            ),
            LintLink::new(LinkKind::External, "https://example.com/", "internal links"),
            LintLink::new(LinkKind::Internal, "/blog/links/", "links"),
        ];
        assert_eq!(LinkTextIsOwnFocus.check(&from, &LintSite::default()), []);
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
