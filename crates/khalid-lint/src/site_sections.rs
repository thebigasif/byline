//! Site rules that weigh a section of one post against another post on the
//! same subject (phase GEO, P9).
//!
//! Two pages on one subject compete for the same searches. When a heading
//! in one post names the whole subject of another post in the same
//! language, one of them should give way: a short post belongs as that
//! section, and a full post should keep the section to a summary that links
//! to it. This follows Koray Tugberk Gubur's macro and micro context, as
//! the ADR "the page or section check matches subject words" records.

use std::collections::{BTreeMap, BTreeSet};

use crate::link_graph::links_to;
use crate::page_or_section::{MIN_STEM_CHARS, same_word, subject_words};
use crate::{Hit, LintPost, RuleId, Severity, SiteRule, SiteView};

/// The letters a word is filed under, so a heading finds the posts whose
/// subject could match it without comparing it with every post. Two words
/// [`same_word`] matches always share them.
fn filed_under(word: &str) -> &str {
    let end = word
        .char_indices()
        .nth(MIN_STEM_CHARS)
        .map_or(word.len(), |(at, _)| at);
    word.get(..end).unwrap_or(word)
}

/// The words that name `post`'s whole subject: its focus keyword, or its
/// title when it has none.
fn subject_of(post: &LintPost) -> Vec<String> {
    let language = post.locale.language();
    let named = post.focus_keyword.as_deref().or(post.title.as_deref());
    named
        .map(|text| subject_words(language, text))
        .unwrap_or_default()
}

/// Whether a heading with the subject words `heading` names the whole
/// subject `subject`. A one-word subject must be the heading's only
/// subject word, so "Links" does not claim every heading about links.
fn names_whole(heading: &[String], subject: &[String]) -> bool {
    let covers = |words: &[String], by: &[String]| {
        words
            .iter()
            .all(|word| by.iter().any(|other| same_word(word, other)))
    };
    !subject.is_empty()
        && covers(subject, heading)
        && (subject.len() > 1 || covers(heading, subject))
}

/// A heading in one post that names the whole subject of another.
#[derive(Debug)]
struct Overlap<'site> {
    /// The post with the section.
    host: &'site LintPost,
    /// The section's heading, as written.
    heading: &'site str,
    /// The post whose subject the heading names.
    other: &'site LintPost,
}

/// Every heading on the site that names the whole subject of another post
/// in the same language, in site order.
fn overlaps(site: &dyn SiteView) -> Vec<Overlap<'_>> {
    let posts = site.posts();
    let subjects: Vec<Vec<String>> = posts.iter().map(subject_of).collect();
    let mut filed: BTreeMap<(&str, &str), Vec<usize>> = BTreeMap::new();
    for (index, (post, subject)) in posts.iter().zip(&subjects).enumerate() {
        if let Some(first) = subject.first() {
            let key = (post.locale.as_str(), filed_under(first));
            filed.entry(key).or_default().push(index);
        }
    }
    let mut found = Vec::new();
    for (at, host) in posts.iter().enumerate() {
        let language = host.locale.language();
        for heading in &host.headings {
            let words = subject_words(language, &heading.text);
            let candidates: BTreeSet<usize> = words
                .iter()
                .filter_map(|word| filed.get(&(host.locale.as_str(), filed_under(word))))
                .flatten()
                .copied()
                .filter(|&index| index != at)
                .collect();
            for index in candidates {
                let (Some(other), Some(subject)) = (posts.get(index), subjects.get(index)) else {
                    continue;
                };
                if names_whole(&words, subject) {
                    found.push(Overlap {
                        host,
                        heading: &heading.text,
                        other,
                    });
                }
            }
        }
    }
    found
}

/// Whether `post` is known to be shorter than `min_words`. A post that does
/// not give its length is not judged.
fn is_short(post: &LintPost, min_words: usize) -> bool {
    post.word_count > 0 && post.word_count < min_words
}

/// **P9: a section that repeats another post.**
///
/// A heading names the whole subject of another post in the same language
/// that is long enough to stand on its own, and the post with the heading
/// never links to it. Reported on the post with the section.
#[derive(Debug)]
pub(crate) struct SectionRepeatsPost {
    /// The thin-post length, from `Thresholds::thin_post_words`.
    pub(crate) min_words: usize,
}

impl SiteRule for SectionRepeatsPost {
    fn id(&self) -> RuleId {
        RuleId::new("section-repeats-post")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check<'site>(&self, site: &'site dyn SiteView) -> Vec<(&'site LintPost, Hit)> {
        overlaps(site)
            .into_iter()
            .filter(|o| !is_short(o.other, self.min_words) && !links_to(o.host, o.other))
            .map(|o| {
                let hit = Hit::about(o.heading).suggesting(vec![o.other.slug.as_str().to_owned()]);
                (o.host, hit)
            })
            .collect()
    }
}

/// **P9: a short post that fits as a section.**
///
/// A post shorter than the thin-post length whose whole subject is already
/// a heading in another post in the same language. Reported on the short
/// post, naming the post it fits into.
#[derive(Debug)]
pub(crate) struct PostFitsSection {
    /// The thin-post length, from `Thresholds::thin_post_words`.
    pub(crate) min_words: usize,
}

impl SiteRule for PostFitsSection {
    fn id(&self) -> RuleId {
        RuleId::new("post-fits-section")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check<'site>(&self, site: &'site dyn SiteView) -> Vec<(&'site LintPost, Hit)> {
        overlaps(site)
            .into_iter()
            .filter(|o| is_short(o.other, self.min_words))
            .map(|o| {
                let hit = Hit::about(format!("\"{}\" in {}", o.heading, o.host.slug.as_str()));
                (o.other, hit)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(text: &str) -> Vec<String> {
        subject_words("en", text)
    }

    #[test]
    fn a_heading_names_a_subject_of_several_words_when_it_holds_them_all() {
        assert!(names_whole(
            &words("Choosing link text"),
            &words("link text")
        ));
        assert!(names_whole(
            &words("Links and their text"),
            &words("link text")
        ));
        assert!(!names_whole(
            &words("Choosing anchors"),
            &words("link text")
        ));
    }

    #[test]
    fn a_one_word_subject_must_be_the_headings_only_subject_word() {
        assert!(names_whole(&words("Why links?"), &words("links")));
        assert!(!names_whole(&words("Choosing link text"), &words("links")));
        assert!(!names_whole(&words("Choosing link text"), &[]));
    }

    #[test]
    fn matching_words_are_filed_under_the_same_letters() {
        assert_eq!(filed_under("link"), filed_under("linking"));
        assert_eq!(filed_under("seo"), "seo");
        assert_eq!(filed_under("übersicht"), "über");
    }
}
