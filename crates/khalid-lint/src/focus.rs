//! The focus topic: the phrase a post wants to be found for.

use khalid_core::{Slug, SlugMode};

use crate::{Hit, LintPost, Rule, RuleId, Severity, SiteView};

/// How many words of the body count as its opening.
const OPENING_WORDS: usize = 100;

/// The words of `text` in lowercase, split at anything that is not a letter
/// or a digit.
fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// Whether `topic` appears in `text` as whole words, in order.
fn holds(text: &[String], topic: &[String]) -> bool {
    !topic.is_empty() && text.windows(topic.len()).any(|run| run == topic)
}

/// Whether the post's address holds the topic as written, or in its plain
/// form, such as `ueber` for "über".
fn in_address(post: &LintPost, focus: &str, topic: &[String]) -> bool {
    let address = words(post.slug.as_str());
    let plain = Slug::from_title(focus, &post.locale, SlugMode::Ascii)
        .map(|slug| words(slug.as_str()))
        .unwrap_or_default();
    holds(&address, topic) || holds(&address, &plain)
}

/// One hit per place the focus topic is missing from: the title, which is
/// also the main headline, the address, the first 100 words, the
/// description, and the headings, where one heading holding it is enough.
/// A place the post does not have yet, such as a missing description, is
/// left to the rule about that place.
#[derive(Debug)]
pub(crate) struct FocusTopicMissing;

impl Rule for FocusTopicMissing {
    fn id(&self) -> RuleId {
        RuleId::new("focus-topic-missing")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        let Some(focus) = post.focus.as_deref() else {
            return Vec::new();
        };
        let topic = words(focus);
        let found = |text: &str| holds(&words(text), &topic);
        let opening: Vec<String> = words(&post.body_text)
            .into_iter()
            .take(OPENING_WORDS)
            .collect();
        let places = [
            ("title", post.title.as_deref().map(found)),
            ("address", Some(in_address(post, focus, &topic))),
            (
                "first 100 words",
                (!opening.is_empty()).then(|| holds(&opening, &topic)),
            ),
            ("description", post.description.as_deref().map(found)),
            (
                "headings",
                (!post.headings.is_empty())
                    .then(|| post.headings.iter().any(|heading| found(&heading.text))),
            ),
        ];
        places
            .into_iter()
            .filter(|(_, present)| *present == Some(false))
            .map(|(place, _)| Hit::about(place))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_topic_matches_whole_words_in_order_only() {
        let text = words("Why Internal-Links matter");
        assert!(holds(&text, &words("internal links")));
        assert!(!holds(&text, &words("links internal")));
        assert!(!holds(&text, &words("intern")));
        assert!(!holds(&text, &[]));
    }
}
