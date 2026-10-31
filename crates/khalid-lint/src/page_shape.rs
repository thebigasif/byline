//! The shape a comparison, alternatives, best-of or review page needs
//! (phase GEO, P4).
//!
//! Each kind of title from [`PageKind`] promises a known shape. Koray
//! Tugberk Gubur's document templates keep every page of one query template
//! alike, and Steve Toth's answer-first passages put the verdict where a
//! reader and an answer engine look first. Neither names these four checks
//! or their limits; they are this project's reading of both.

use std::collections::BTreeMap;

use crate::page_or_section::{same_word, subject_words};
use crate::{Hit, LintPost, PageKind, Rule, RuleId, Severity, SiteView};

/// The kind of page `post`'s title asks for.
fn kind(post: &LintPost) -> Option<PageKind> {
    let title = post.title.as_deref()?;
    PageKind::of_title(post.locale.language(), title)
}

/// A page that weighs choices against each other with no table.
#[derive(Debug)]
pub(crate) struct ComparisonWithoutTable;

impl Rule for ComparisonWithoutTable {
    fn id(&self) -> RuleId {
        RuleId::new("comparison-without-table")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        let compares = matches!(
            kind(post),
            Some(
                PageKind::Comparison { .. }
                    | PageKind::Alternatives { .. }
                    | PageKind::BestOf { .. }
            )
        );
        if !compares || post.tables > 0 {
            return Vec::new();
        }
        vec![
            Hit::about("the post weighs choices against each other in paragraphs only").suggesting(
                vec![
                    "add a table with one column per choice and one row per point you compare"
                        .to_owned(),
                ],
            ),
        ]
    }
}

/// A comparison whose opening never names one of the two things compared.
#[derive(Debug)]
pub(crate) struct ComparisonAnswerLate;

impl Rule for ComparisonAnswerLate {
    fn id(&self) -> RuleId {
        RuleId::new("comparison-answer-late")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        let Some(PageKind::Comparison { first, second }) = kind(post) else {
            return Vec::new();
        };
        let language = post.locale.language();
        let opening: Vec<String> = opening(post)
            .iter()
            .flat_map(|line| subject_words(language, line))
            .collect();
        [first, second]
            .into_iter()
            .filter(|side| {
                // A side of filler words only, such as "A", is never reported.
                let named = subject_words(language, side);
                !named.is_empty()
                    && !named
                        .iter()
                        .any(|word| opening.iter().any(|own| same_word(word, own)))
            })
            .map(|side| {
                Hit::about(format!("the opening never names {side}")).suggesting(vec![
                    "open with a paragraph that names both choices and says which suits which reader"
                        .to_owned(),
                ])
            })
            .collect()
    }
}

/// The plain lines of `post` before its first heading.
fn opening(post: &LintPost) -> Vec<&str> {
    post.body_text
        .lines()
        .map(str::trim)
        .take_while(|line| !post.headings.iter().any(|heading| heading.text == *line))
        .filter(|line| !line.is_empty())
        .collect()
}

/// A list whose title promises more picks than any one heading level holds.
#[derive(Debug)]
pub(crate) struct FewerPicksThanTitle;

impl Rule for FewerPicksThanTitle {
    fn id(&self) -> RuleId {
        RuleId::new("fewer-picks-than-title")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        let Some(
            PageKind::Alternatives {
                picks: Some(promised),
            }
            | PageKind::BestOf {
                picks: Some(promised),
            },
        ) = kind(post)
        else {
            return Vec::new();
        };
        let mut per_level = BTreeMap::new();
        for heading in &post.headings {
            *per_level.entry(heading.level).or_insert(0_usize) += 1;
        }
        let most = per_level.into_values().max().unwrap_or(0);
        if most >= promised {
            return Vec::new();
        }
        vec![
            Hit::about(format!(
                "the title promises {promised} picks, but no heading level holds more than {most}"
            ))
            .suggesting(vec![
                "give each pick its own heading, or change the number in the title".to_owned(),
            ]),
        ]
    }
}

/// A review with no review block, so it gives no rating.
#[derive(Debug)]
pub(crate) struct ReviewWithoutRating;

impl Rule for ReviewWithoutRating {
    fn id(&self) -> RuleId {
        RuleId::new("review-without-rating")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        if kind(post) != Some(PageKind::Review) || post.reviews > 0 {
            return Vec::new();
        }
        vec![Hit::about("the review gives no rating").suggesting(vec![
            "add a review block with the item, your score and a short verdict".to_owned(),
        ])]
    }
}

#[cfg(test)]
mod tests {
    use khalid_core::{Locale, Slug};

    use super::*;
    use crate::{Heading, HeadingLevel, LintSite};

    fn post(title: &str, body: &str, headings: &[&str]) -> LintPost {
        let mut post = LintPost::new(
            Locale::try_new("en").unwrap(),
            Slug::try_new("a-post").unwrap(),
        );
        post.title = Some(title.to_owned());
        post.body_text = body.to_owned();
        post.headings = headings
            .iter()
            .map(|text| Heading::new(HeadingLevel::H2, *text))
            .collect();
        post
    }

    fn hits(rule: &dyn Rule, post: &LintPost) -> Vec<String> {
        rule.check(post, &LintSite::default())
            .into_iter()
            .map(|hit| hit.subject().unwrap_or_default().to_owned())
            .collect()
    }

    #[test]
    fn a_table_settles_a_comparison_and_other_titles_need_none() {
        let mut compared = post("Ahrefs vs Semrush", "Both.", &[]);
        assert_eq!(hits(&ComparisonWithoutTable, &compared).len(), 1);
        compared.tables = 1;
        assert_eq!(
            hits(&ComparisonWithoutTable, &compared),
            Vec::<String>::new()
        );
        let plain = post("Why internal links matter", "Text.", &[]);
        assert_eq!(hits(&ComparisonWithoutTable, &plain), Vec::<String>::new());
    }

    #[test]
    fn the_opening_must_name_both_sides_before_the_first_heading() {
        let headings = ["Price"];
        let both = post(
            "Ahrefs vs Semrush",
            "Pick Ahrefs for links, Semrush for ads.\nPrice\nSemrush",
            &headings,
        );
        assert_eq!(hits(&ComparisonAnswerLate, &both), Vec::<String>::new());
        let one = post(
            "Ahrefs vs Semrush",
            "Ahrefs wins for links.\nPrice\nSemrush costs more.",
            &headings,
        );
        let found = hits(&ComparisonAnswerLate, &one);
        assert_eq!(found, ["the opening never names Semrush"]);
        let filler = post("A vs Semrush", "Semrush wins.\nPrice", &headings);
        assert_eq!(hits(&ComparisonAnswerLate, &filler), Vec::<String>::new());
    }

    #[test]
    fn a_numbered_list_needs_as_many_headings_at_one_level() {
        let short = post("The 3 best SEO tools", "Text.", &["Ahrefs", "Semrush"]);
        assert_eq!(hits(&FewerPicksThanTitle, &short).len(), 1);
        let full = post(
            "The 3 best SEO tools",
            "Text.",
            &["Ahrefs", "Semrush", "Moz"],
        );
        assert_eq!(hits(&FewerPicksThanTitle, &full), Vec::<String>::new());
        let open = post("The best SEO tools", "Text.", &[]);
        assert_eq!(hits(&FewerPicksThanTitle, &open), Vec::<String>::new());
    }

    #[test]
    fn a_review_needs_a_review_block() {
        let mut review = post("Ahrefs review", "Text.", &[]);
        assert_eq!(hits(&ReviewWithoutRating, &review).len(), 1);
        review.reviews = 1;
        assert_eq!(hits(&ReviewWithoutRating, &review), Vec::<String>::new());
    }
}
