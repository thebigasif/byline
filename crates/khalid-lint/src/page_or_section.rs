//! The page or section check (phase GEO, P9).
//!
//! A page keeps to one main subject, and each section serves that subject.
//! A section long enough to stand as a post of its own that never names the
//! post's subject has crossed that border: it belongs on its own page, linked
//! from here. This follows Koray Tugberk Gubur's macro and micro context: one
//! main context per page, with sections as its sub-contexts. Neither he nor
//! Steve Toth gives a word count or a way to match words, so those are this
//! project's choices: a section is long enough when it would pass the
//! thin-post rule, and it names the subject when it shares a word with the
//! title, the search title or the keywords.

use crate::focus::words;
use crate::words::names_subject;
use crate::{HeadingLevel, Hit, LintPost, Rule, RuleId, Severity, SiteView};

/// The fewest letters a word needs before a longer word that starts with it
/// counts as the same word, so "link" matches "links" but "on" never
/// matches "online".
const MIN_STEM_CHARS: usize = 4;

/// The most letters an ending may add, such as "s", "en" or "ing".
const MAX_ENDING_CHARS: usize = 3;

/// The words of `text` that can name a subject in `language`, in lowercase.
pub(crate) fn subject_words(language: &str, text: &str) -> Vec<String> {
    words(text)
        .into_iter()
        .filter(|word| names_subject(language, word))
        .collect()
}

/// Whether two lowercase words are the same word, allowing a short ending,
/// as "link" and "links" or "Bild" and "Bilder" are.
pub(crate) fn same_word(a: &str, b: &str) -> bool {
    let (short, long) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    let short_chars = short.chars().count();
    let ending = long.chars().count().saturating_sub(short_chars);
    short == long
        || (short_chars >= MIN_STEM_CHARS && ending <= MAX_ENDING_CHARS && long.starts_with(short))
}

/// The words that name `post`'s subject: those of its title, search title,
/// focus keyword and secondary keywords.
fn post_subject(post: &LintPost) -> Vec<String> {
    let language = post.locale.language();
    let named = [&post.title, &post.seo_title, &post.focus_keyword];
    let texts = named.into_iter().flatten().chain(&post.secondary_keywords);
    texts
        .flat_map(|text| subject_words(language, text))
        .collect()
}

/// One `##` section of a post: its heading and the plain lines under it, up
/// to the next `##`. Deeper headings inside it belong to it.
#[derive(Debug)]
struct Section<'a> {
    heading: &'a str,
    lines: Vec<&'a str>,
}

/// The `##` sections of `post`, in body order. Text before the first `##`
/// is the introduction and belongs to none.
fn sections(post: &LintPost) -> Vec<Section<'_>> {
    let heading_level = |line: &str| {
        post.headings
            .iter()
            .find(|heading| heading.text == line)
            .map(|heading| heading.level)
    };
    let mut sections: Vec<Section<'_>> = Vec::new();
    for line in post.body_text.lines().map(str::trim) {
        match heading_level(line) {
            Some(HeadingLevel::H2) => sections.push(Section {
                heading: line,
                lines: Vec::new(),
            }),
            Some(_) => {}
            None if line.is_empty() => {}
            None => {
                if let Some(section) = sections.last_mut() {
                    section.lines.push(line);
                }
            }
        }
    }
    sections
}

/// **P9: a section that wants its own page.**
///
/// A `##` section of at least `min_words` words whose heading and text never
/// name the post's subject.
#[derive(Debug)]
pub(crate) struct SectionWantsOwnPage {
    /// The fewest words, from `Thresholds::thin_post_words`: a section that
    /// long would pass as a post of its own.
    pub(crate) min_words: usize,
}

impl Rule for SectionWantsOwnPage {
    fn id(&self) -> RuleId {
        RuleId::new("section-wants-own-page")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        let subject = post_subject(post);
        if subject.is_empty() {
            return Vec::new();
        }
        let language = post.locale.language();
        let names_it = |text: &str| {
            subject_words(language, text)
                .iter()
                .any(|word| subject.iter().any(|own| same_word(word, own)))
        };
        let mut hits = Vec::new();
        for section in sections(post) {
            let count: usize = section
                .lines
                .iter()
                .map(|line| line.split_whitespace().count())
                .sum();
            if count < self.min_words
                || names_it(section.heading)
                || section.lines.iter().any(|line| names_it(line))
            {
                continue;
            }
            hits.push(
                Hit::about(format!(
                    "section \"{}\" runs {count} words without naming the post's subject",
                    section.heading
                ))
                .suggesting(vec![
                    "move it to its own post and link to it from here, or tie it to the post's subject in its heading and text"
                        .to_owned(),
                ]),
            );
        }
        hits
    }
}

#[cfg(test)]
mod tests {
    use khalid_core::{Locale, Slug};

    use super::*;
    use crate::{Heading, LintSite};

    /// A post titled `title` whose body is `## Away` with `words` copies
    /// of `word`.
    fn post(locale: &str, title: &str, word: &str, words: usize) -> LintPost {
        let mut post = LintPost::new(
            Locale::try_new(locale).unwrap(),
            Slug::try_new("a-post").unwrap(),
        );
        post.title = Some(title.to_owned());
        post.headings = vec![Heading::new(HeadingLevel::H2, "Away")];
        post.body_text = format!("Intro.\nAway\n{}", vec![word; words].join(" "));
        post
    }

    fn hits(post: &LintPost) -> usize {
        let rule = SectionWantsOwnPage { min_words: 300 };
        rule.check(post, &LintSite::default()).len()
    }

    #[test]
    fn a_long_section_off_the_subject_is_reported() {
        assert_eq!(
            hits(&post("en", "Why internal links matter", "hosting", 300)),
            1
        );
    }

    #[test]
    fn a_short_section_off_the_subject_is_left_alone() {
        assert_eq!(
            hits(&post("en", "Why internal links matter", "hosting", 299)),
            0
        );
    }

    #[test]
    fn a_plural_or_an_ending_still_names_the_subject() {
        assert_eq!(
            hits(&post("en", "Why internal links matter", "link", 300)),
            0
        );
        assert_eq!(hits(&post("de", "Bilder richtig benennen", "Bild", 300)), 0);
    }

    #[test]
    fn question_words_and_fillers_never_name_the_subject() {
        assert_eq!(hits(&post("en", "How to do it", "how", 300)), 0);
        assert_eq!(hits(&post("en", "How to host a blog", "how", 300)), 1);
        assert_eq!(
            hits(&post("de", "Warum Bilder wichtig sind", "warum", 300)),
            1
        );
    }

    #[test]
    fn a_secondary_keyword_names_the_subject() {
        let mut post = post("en", "Why internal links matter", "hosting", 300);
        post.secondary_keywords = vec!["web hosting".to_owned()];
        assert_eq!(hits(&post), 0);
    }

    #[test]
    fn a_short_word_matches_only_itself() {
        assert!(same_word("seo", "seo"));
        assert!(!same_word("seo", "seos"));
        assert!(same_word("link", "linking"));
        assert!(!same_word("link", "linkedness"));
    }
}
