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
        let Some(keyword) = post.focus_keyword.as_deref().map(str::trim) else {
            return Vec::new();
        };
        if self.0.holds(post, keyword, &words(keyword)) == Some(false) {
            vec![Hit::about(keyword)]
        } else {
            Vec::new()
        }
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
}
