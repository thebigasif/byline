//! The focus keyword: the phrase a post wants to be found for, and the
//! places a reader and Google look for it.

use khalid_core::{Slug, SlugMode};

use crate::{Hit, LintPost, Rule, RuleId, Severity, SiteView};

/// The words of `text` in lowercase, split at anything that is not a letter
/// or a digit.
pub(crate) fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// Whether `topic` appears in `text` as whole words, in order.
pub(crate) fn holds(text: &[String], topic: &[String]) -> bool {
    !topic.is_empty() && text.windows(topic.len()).any(|run| run == topic)
}

/// A place in a post where the focus keyword belongs, each checked by its
/// own rule with its own message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Place {
    /// The title, which is also the page's main headline.
    Title,
    /// The description under the title in search results.
    Description,
    /// The first paragraph of the body.
    FirstParagraph,
    /// The subheadings, where one holding the keyword is enough.
    Subheading,
    /// The last part of the web address.
    Address,
    /// The image descriptions, where one holding the keyword is enough.
    ImageAlt,
}

impl Place {
    /// The rule that checks this place.
    const fn rule(self) -> RuleId {
        RuleId::new(match self {
            Self::Title => "focus-not-in-title",
            Self::Description => "focus-not-in-description",
            Self::FirstParagraph => "focus-not-in-first-paragraph",
            Self::Subheading => "focus-not-in-subheading",
            Self::Address => "focus-not-in-address",
            Self::ImageAlt => "focus-not-in-image-alt",
        })
    }

    /// Whether this place in `post` holds `keyword`, whose words are
    /// `topic`, or `None` when the post does not have the place yet, which
    /// is left to the rule about that place.
    fn holds(self, post: &LintPost, keyword: &str, topic: &[String]) -> Option<bool> {
        let found = |text: &str| holds(&words(text), topic);
        match self {
            Self::Title => post.title.as_deref().map(found),
            Self::Description => post.description.as_deref().map(found),
            Self::FirstParagraph => first_paragraph(post).map(found),
            Self::Subheading => (!post.headings.is_empty())
                .then(|| post.headings.iter().any(|heading| found(&heading.text))),
            Self::Address => Some(in_address(post, keyword, topic)),
            Self::ImageAlt => {
                let mut described = post
                    .images
                    .iter()
                    .filter_map(|image| image.alt.as_deref())
                    .filter(|alt| !alt.trim().is_empty())
                    .peekable();
                described.peek().is_some().then(|| described.any(found))
            }
        }
    }
}

/// The first line of the body's text that is not a heading. The plain text
/// holds one line per paragraph, heading or list item.
fn first_paragraph(post: &LintPost) -> Option<&str> {
    post.body_text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .find(|line| !post.headings.iter().any(|h| h.text.trim() == *line))
}

/// Whether the post's address holds the keyword as written, or in its plain
/// form, such as `ueber` for "über".
fn in_address(post: &LintPost, keyword: &str, topic: &[String]) -> bool {
    let address = words(post.slug.as_str());
    let plain = Slug::from_title(keyword, &post.locale, SlugMode::Ascii)
        .map(|slug| words(slug.as_str()))
        .unwrap_or_default();
    holds(&address, topic) || holds(&address, &plain)
}

/// The post's focus keyword without outer spaces, or `None` when it has
/// none or it holds no word, as a draft from another caller may.
fn focus_keyword(post: &LintPost) -> Option<&str> {
    post.focus_keyword
        .as_deref()
        .filter(|keyword| !words(keyword).is_empty())
        .map(str::trim)
}

/// The focus keyword is missing from one place in the post. The finding
/// names the keyword, so the writer sees which words to add.
#[derive(Debug)]
pub(crate) struct FocusNotIn(pub(crate) Place);

impl Rule for FocusNotIn {
    fn id(&self) -> RuleId {
        self.0.rule()
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        let Some(keyword) = focus_keyword(post) else {
            return Vec::new();
        };
        if self.0.holds(post, keyword, &words(keyword)) == Some(false) {
            vec![Hit::about(keyword)]
        } else {
            Vec::new()
        }
    }
}

/// The fewest words a post needs before its keyword density is measured,
/// because one use more or less swings the share of a short text too far.
const DENSITY_MIN_WORDS: usize = 100;

/// The post's focus keyword and how many times per thousand words its body
/// holds it, or `None` when it has no keyword or too few words to judge.
fn per_thousand(post: &LintPost) -> Option<(&str, usize)> {
    let keyword = focus_keyword(post)?;
    let text = words(&post.body_text);
    if text.len() < DENSITY_MIN_WORDS {
        return None;
    }
    let topic = words(keyword);
    let uses = text
        .windows(topic.len())
        .filter(|run| *run == topic)
        .count();
    Some((keyword, uses * 1000 / text.len()))
}

/// The focus keyword appears fewer times per thousand words than the
/// floor, so the text does not read as being about it.
#[derive(Debug)]
pub(crate) struct FocusTooRare {
    /// From `Thresholds::focus_min_per_thousand_words`.
    pub(crate) min: usize,
}

impl Rule for FocusTooRare {
    fn id(&self) -> RuleId {
        RuleId::new("focus-too-rare")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        per_thousand(post)
            .filter(|&(_, rate)| rate < self.min)
            .map(|(keyword, _)| Hit::about(keyword))
            .into_iter()
            .collect()
    }
}

/// The focus keyword appears more times per thousand words than the
/// ceiling, so the text reads as stuffed.
#[derive(Debug)]
pub(crate) struct FocusTooOften {
    /// From `Thresholds::focus_max_per_thousand_words`.
    pub(crate) max: usize,
}

impl Rule for FocusTooOften {
    fn id(&self) -> RuleId {
        RuleId::new("focus-too-often")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        per_thousand(post)
            .filter(|&(_, rate)| rate > self.max)
            .map(|(keyword, _)| Hit::about(keyword))
            .into_iter()
            .collect()
    }
}

/// A secondary keyword the body never uses, one hit per keyword in the
/// header's order.
#[derive(Debug)]
pub(crate) struct SecondaryKeywordUnused;

impl Rule for SecondaryKeywordUnused {
    fn id(&self) -> RuleId {
        RuleId::new("secondary-keyword-unused")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        let text = words(&post.body_text);
        post.secondary_keywords
            .iter()
            .map(|keyword| keyword.trim())
            .filter(|keyword| {
                let topic = words(keyword);
                !topic.is_empty() && !holds(&text, &topic)
            })
            .map(Hit::about)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use khalid_core::Locale;

    use super::*;
    use crate::{Heading, HeadingLevel};

    #[test]
    fn a_topic_matches_whole_words_in_order_only() {
        let text = words("Why Internal-Links matter");
        assert!(holds(&text, &words("internal links")));
        assert!(!holds(&text, &words("links internal")));
        assert!(!holds(&text, &words("intern")));
        assert!(!holds(&text, &[]));
    }

    #[test]
    fn the_first_paragraph_skips_headings_and_blank_lines() {
        let mut post = LintPost::new(Locale::try_new("en").unwrap(), Slug::try_new("a").unwrap());
        post.body_text = "\nWhy links\n\nLinks help.\nMore.".to_owned();
        post.headings = vec![Heading::new(HeadingLevel::H2, "Why links")];
        assert_eq!(first_paragraph(&post), Some("Links help."));
    }

    #[test]
    fn a_german_keyword_matches_its_plain_address() {
        let mut post = LintPost::new(
            Locale::try_new("de").unwrap(),
            Slug::try_new("ueber-interne-links").unwrap(),
        );
        post.focus_keyword = Some("über interne Links".to_owned());
        let hits = FocusNotIn(Place::Address).check(&post, &crate::LintSite::default());
        assert_eq!(hits, []);
    }

    #[test]
    fn a_keyword_without_words_is_no_keyword() {
        let mut post = LintPost::new(Locale::try_new("en").unwrap(), Slug::try_new("a").unwrap());
        post.focus_keyword = Some(" - ".to_owned());
        post.secondary_keywords = vec![String::new()];
        post.title = Some("A title".to_owned());
        post.body_text = ["word"; 120].join(" ");
        let site = crate::LintSite::default();
        assert_eq!(FocusNotIn(Place::Title).check(&post, &site), []);
        assert_eq!(FocusTooRare { min: 5 }.check(&post, &site), []);
        assert_eq!(SecondaryKeywordUnused.check(&post, &site), []);
    }
}
