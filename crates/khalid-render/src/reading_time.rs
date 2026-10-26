//! How long a post takes to read, shown as "8 min read" beside its date.

use khalid_core::Locale;
use serde::Serialize;

use crate::BodyHtml;

/// Words an adult reads in a minute, the rate most reading-time estimates
/// use. A higher number shows shorter times.
const WORDS_PER_MINUTE: usize = 230;

/// Characters a reader of Japanese or Chinese reads in a minute. These
/// languages have no spaces between words, so characters are counted.
const CHARACTERS_PER_MINUTE: usize = 500;

/// The languages written without spaces between words, by language code.
const WITHOUT_SPACES: [&str; 6] = ["ja", "zh", "th", "lo", "km", "my"];

/// A post's reading time in whole minutes, never less than one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ReadingTime {
    minutes: u32,
}

impl ReadingTime {
    /// The reading time of `body` in `locale`'s language: its words at
    /// about 230 a minute, or for a language written without spaces, its
    /// characters at about 500 a minute, rounded up. Markup is not counted.
    pub fn of(body: &BodyHtml, locale: &Locale) -> Self {
        let text = text_of(body.as_str());
        let (count, per_minute) = if WITHOUT_SPACES.contains(&locale.language()) {
            let characters = text.chars().filter(|c| !c.is_whitespace()).count();
            (characters, CHARACTERS_PER_MINUTE)
        } else {
            (text.split_whitespace().count(), WORDS_PER_MINUTE)
        };
        let minutes = count.div_ceil(per_minute).max(1);
        Self {
            minutes: u32::try_from(minutes).unwrap_or(u32::MAX),
        }
    }

    /// The whole minutes.
    pub fn minutes(self) -> u32 {
        self.minutes
    }
}

/// The text of `html` with every tag replaced by a space, so words in two
/// elements stay two words.
fn text_of(html: &str) -> String {
    let mut text = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => {
                in_tag = false;
                text.push(' ');
            }
            _ if !in_tag => text.push(c),
            _ => {}
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minutes(html: &str, language: &str) -> u32 {
        let body = BodyHtml::from_sanitised(html.to_owned());
        ReadingTime::of(&body, &Locale::try_new(language).unwrap()).minutes()
    }

    fn paragraph(words: usize) -> String {
        format!("<p>{}</p>", vec!["word"; words].join(" "))
    }

    #[test]
    fn rounds_up_at_two_hundred_and_thirty_words_a_minute() {
        assert_eq!(minutes(&paragraph(230), "en"), 1);
        assert_eq!(minutes(&paragraph(231), "en"), 2);
        assert_eq!(minutes(&paragraph(1840), "de"), 8);
    }

    #[test]
    fn a_short_or_empty_post_still_takes_a_minute() {
        assert_eq!(minutes("<p>Hi.</p>", "en"), 1);
        assert_eq!(minutes("", "en"), 1);
    }

    #[test]
    fn markup_is_not_counted_and_does_not_join_words() {
        let html =
            r#"<h2 id="what-it-does">What</h2><p>it<a href="/blog/a-long-address/">does</a></p>"#;
        assert_eq!(text_of(html).split_whitespace().count(), 3);
    }

    #[test]
    fn japanese_and_chinese_count_characters() {
        let japanese = format!("<p>{}</p>", "読".repeat(1000));
        assert_eq!(minutes(&japanese, "ja"), 2);
        let chinese = format!("<p>{}</p>", "读".repeat(1001));
        assert_eq!(minutes(&chinese, "zh-Hans"), 3);
        assert_eq!(minutes(&japanese, "en"), 1, "one long word in English");
    }
}
