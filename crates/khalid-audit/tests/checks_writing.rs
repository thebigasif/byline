//! The AI search checks on how a page's own text is written: the answer
//! under a question heading and paragraph length (phase GEO, A1, built on
//! W1 and W2).

pub mod support;

use khalid_audit::{Check, Severity, Thresholds, audit};
use support::{HOME, crawled, page};

/// The checks this file is about.
const CHECKS: [Check; 2] = [Check::AnswerCapsule, Check::ParagraphTooLong];

/// `count` plain words.
#[cfg(test)]
fn words(count: usize) -> String {
    vec!["word"; count].join(" ")
}

/// This file's checks on one page with `body`, measured with `thresholds`.
#[cfg(test)]
async fn found(body: &str, thresholds: Thresholds) -> Vec<(Check, String)> {
    let crawl = crawled(&[(HOME, page("<title>Rye</title>", body))]).await;
    audit(&crawl, thresholds)
        .iter()
        .filter(|issue| CHECKS.contains(&issue.check()))
        .map(|issue| (issue.check(), issue.detail().to_owned()))
        .collect()
}

#[tokio::test]
async fn a_question_answered_in_40_to_60_words_passes() {
    // The answer is every paragraph up to the next heading, as the
    // linter's W1 rule counts it, so a post the linter passes passes here.
    let body = format!(
        "<h1>Rye</h1><h2>How long does rye take?</h2><p>{}</p><p>{}</p>\
         <h2>Flour</h2><p>{}</p>",
        words(30),
        words(20),
        words(100)
    );
    assert_eq!(found(&body, Thresholds::default()).await, []);
}

#[tokio::test]
async fn only_articles_and_pages_that_do_not_say_are_judged() {
    let body = format!(
        "<h2>Who writes here?</h2><p>Asif, {} 2 posts.</p>",
        words(3)
    );
    let checks = |head: &'static str| {
        let body = body.clone();
        async move {
            let crawl = crawled(&[(HOME, page(head, &body))]).await;
            audit(&crawl, Thresholds::default())
                .iter()
                .filter(|issue| CHECKS.contains(&issue.check()))
                .count()
        }
    };
    assert_eq!(
        checks(r#"<meta property="og:type" content="website">"#).await,
        0
    );
    assert_eq!(
        checks(r#"<meta property="og:type" content="article">"#).await,
        1
    );
    assert_eq!(checks("").await, 1);
}

#[tokio::test]
async fn a_missing_short_or_long_answer_is_named_with_its_heading() {
    let body = format!(
        "<h2>Why rye?</h2><h3>Flour</h3><p>{}</p>\
         <h2>Is it hard?</h2><p>{}</p>\
         <h2>What about wheat?</h2><p>{}</p>\
         <h2>Not a question</h2><p>Short.</p>",
        words(80),
        words(12),
        words(61)
    );
    assert_eq!(
        found(&body, Thresholds::default()).await,
        [
            (
                Check::AnswerCapsule,
                "0 words under \"Why rye?\"".to_owned()
            ),
            (
                Check::AnswerCapsule,
                "12 words under \"Is it hard?\"".to_owned()
            ),
            (
                Check::AnswerCapsule,
                "61 words under \"What about wheat?\"".to_owned()
            ),
        ]
    );
    assert_eq!(Check::AnswerCapsule.severity(), Severity::Warning);
}

#[tokio::test]
async fn the_answer_length_and_the_paragraph_limit_can_change() {
    let body = format!("<h2>Is it hard?</h2><p>{}</p>", words(12));
    let thresholds = Thresholds::default()
        .with_answer_words(10, 20)
        .with_paragraph_words(11);
    assert_eq!(
        found(&body, thresholds).await,
        [(
            Check::ParagraphTooLong,
            "12 words: word word word word word word word word ...".to_owned()
        )]
    );
}

#[tokio::test]
async fn a_paragraph_past_225_words_is_named() {
    let body = format!("<p>{}</p><p>{}</p>", words(225), words(226));
    let checks: Vec<Check> = found(&body, Thresholds::default())
        .await
        .into_iter()
        .map(|(check, _)| check)
        .collect();
    assert_eq!(checks, [Check::ParagraphTooLong]);
}
