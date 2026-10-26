//! The messages test from `docs/standards/testing.md` (BL-TST-4, BL-TXT-2).
//!
//! Every message in the catalogue is three sentences, each under 25 words,
//! with no em dash, no jargon word, and a fix that starts with an action.

use khalid_lint::{Catalogue, Message};

/// Words a writer who is new to SEO would not know. Text inside backticks is
/// not checked, because it names what the writer types.
const JARGON: &[&str] = &[
    "anchor",
    "backlink",
    "canonical",
    "crawler",
    "ctr",
    "dom",
    "h1",
    "h2",
    "h3",
    "hreflang",
    "indexation",
    "metadata",
    "nofollow",
    "pagerank",
    "schema",
    "serp",
    "slug",
];

/// The verbs a fix may start with: each one is something the writer does.
const ACTIONS: &[&str] = &[
    "Add", "Change", "Cut", "Delete", "Describe", "Give", "Link", "Merge", "Move", "Remove",
    "Rename", "Replace", "Rewrite", "Shorten", "Split", "Turn", "Update", "Use", "Write",
];

/// The text of `sentence` with every backtick span removed.
fn outside_code(sentence: &str) -> String {
    sentence.split('`').step_by(2).collect::<Vec<_>>().join(" ")
}

fn check_sentence(rule: &str, part: &str, sentence: &str) {
    let plain = outside_code(sentence);
    assert!(
        sentence.ends_with('.'),
        "{rule} {part}: end with a full stop"
    );
    let inner = plain.trim_end().trim_end_matches('.');
    assert!(
        !inner.contains(". ") && !inner.contains('?') && !inner.contains('!'),
        "{rule} {part}: one sentence only, got {sentence:?}"
    );
    let words = sentence.split_whitespace().count();
    assert!(words < 25, "{rule} {part}: {words} words, keep it under 25");
    assert!(!sentence.contains('\u{2014}'), "{rule} {part}: no em dash");
    for word in plain.split(|c: char| !c.is_alphanumeric()) {
        let lower = word.to_lowercase();
        assert!(
            !JARGON.contains(&lower.as_str()),
            "{rule} {part}: replace the jargon word {word:?} with plain words"
        );
    }
}

fn check_message(rule: &str, message: &Message) {
    check_sentence(rule, "what", message.what());
    check_sentence(rule, "why", message.why());
    check_sentence(rule, "fix", message.fix());
    let verb = message.fix().split_whitespace().next().unwrap_or_default();
    assert!(
        ACTIONS.contains(&verb),
        "{rule} fix: start with an action such as Add or Change, got {verb:?}"
    );
}

#[test]
fn every_catalogue_message_is_three_plain_sentences() {
    let catalogue = Catalogue::builtin().unwrap();
    assert!(!catalogue.is_empty(), "the catalogue has messages");
    for (rule, message) in &catalogue {
        check_message(rule, message);
        assert!(!message.guide().as_str().is_empty(), "{rule}: guide slug");
    }
}

#[test]
#[should_panic(expected = "one sentence only")]
fn the_check_refuses_two_sentences_in_one_part() {
    check_sentence("example", "why", "Dates matter. Readers check them.");
}

#[test]
#[should_panic(expected = "jargon")]
fn the_check_refuses_jargon_outside_backticks() {
    check_sentence("example", "why", "Your H1 tells Google the topic.");
}

#[test]
fn the_check_allows_jargon_inside_backticks() {
    check_sentence("example", "fix", "Change `slug:` in the header.");
}

#[test]
#[should_panic(expected = "under 25")]
fn the_check_refuses_long_sentences() {
    let long = format!("{}.", ["word"; 25].join(" "));
    check_sentence("example", "why", &long);
}
