//! A comment that cannot be sent gets a plain page in the reader's
//! language, with the form filled in again.

use khalid_comments::{CommentForm, FieldProblem, Refusal, RefusalPages};
use khalid_core::Locale;

const LINKS: (&str, &str) = ("/blog/comments/", "/blog/assets/site.css");

#[cfg(test)]
fn sent() -> CommentForm {
    CommentForm {
        page: "/blog/why-links-matter/".to_owned(),
        parent: Some("4".to_owned()),
        name: "Lena \"<b>\"".to_owned(),
        email: "lena@".to_owned(),
        comment: "I <3 this & that".to_owned(),
        website: String::new(),
    }
}

#[cfg(test)]
fn page(language: &str, refusal: &Refusal) -> String {
    let pages = RefusalPages::load().unwrap();
    pages.page(
        &language.parse::<Locale>().unwrap(),
        refusal,
        &sent(),
        LINKS,
    )
}

#[test]
fn every_problem_is_a_sentence_and_the_form_comes_back_filled_in() {
    let refusal = Refusal::Fields(vec![FieldProblem::NameTooLong, FieldProblem::EmailInvalid]);
    let html = page("en", &refusal);
    assert!(html.starts_with("<!doctype html>\n<html lang=\"en\">"));
    assert!(html.contains("<meta name=\"robots\" content=\"noindex\">"));
    assert!(html.contains("<link rel=\"stylesheet\" href=\"/blog/assets/site.css\">"));
    assert_eq!(html.matches("<h1>").count(), 1);
    assert!(html.contains("<li>Please shorten your name to 80 letters.</li>"));
    assert!(html.contains("<li>Please check your email address, or leave it empty.</li>"));
    assert!(html.contains("action=\"/blog/comments/\""));
    assert!(html.contains("name=\"page\" value=\"/blog/why-links-matter/\""));
    assert!(
        html.contains("name=\"parent\" value=\"4\""),
        "a reply stays a reply"
    );
    assert!(html.contains("value=\"Lena &quot;&lt;b&gt;&quot;\""));
    assert!(html.contains(">I &lt;3 this &amp; that</textarea>"));
    assert!(html.contains("<a href=\"/blog/why-links-matter/\">Back to the post</a>"));
    assert!(!html.contains("<script"), "the page needs no script");
}

#[test]
fn a_reply_to_a_hidden_comment_comes_back_on_its_own() {
    let html = page("en", &Refusal::ParentGone);
    assert!(html.contains("The comment you answered is no longer shown"));
    assert!(!html.contains("name=\"parent\""));
}

#[test]
fn a_reader_is_told_how_long_to_wait() {
    let html = page("de", &Refusal::TooMany { minutes: 7 });
    assert!(html.contains("<html lang=\"de\">"));
    assert!(html.contains("versuche es in 7 Minuten noch einmal"));
    assert!(html.contains("Zurück zum Beitrag"));
}

#[test]
fn a_page_that_takes_no_comments_shows_no_form() {
    let html = page("en", &Refusal::NoPost);
    assert!(html.contains("<li>This page does not take comments.</li>"));
    assert!(!html.contains("<form"));
}

#[test]
fn a_language_without_words_reads_english() {
    let html = page("fr", &Refusal::Unavailable);
    assert!(html.contains("<html lang=\"fr\">"));
    assert!(html.contains("Comments cannot be sent right now"));
}

#[test]
fn every_language_file_has_the_same_words_and_no_em_dash() {
    let english = include_str!("../strings/en.toml");
    let german = include_str!("../strings/de.toml");
    let keys = |text: &str| -> Vec<String> {
        text.lines()
            .filter_map(|line| line.split_once(" = ").map(|(key, _)| key.to_owned()))
            .collect()
    };
    assert_eq!(keys(english), keys(german));
    assert!(!english.contains('\u{2014}') && !german.contains('\u{2014}'));
}
