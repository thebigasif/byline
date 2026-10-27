//! Snippets: a short stretch of a page's text around what matched, with
//! each matching word marked as it was written.

use khalid_search::{Analyzer, Completion, Language, Query, Snippet, SnippetPart};

/// The snippet of `text` for `typed`, written with each mark in brackets.
#[cfg(test)]
fn marked(language: Language, typed: &str, completion: Completion, text: &str) -> String {
    let analyzer = Analyzer::new(language);
    let query = Query::parse(&analyzer, typed, completion).unwrap();
    let snippet = Snippet::of(&analyzer, &query, text);
    snippet
        .parts()
        .iter()
        .map(|part| match part {
            SnippetPart::Match(text) => format!("[{text}]"),
            SnippetPart::Text(text) => text.clone(),
        })
        .collect()
}

#[cfg(test)]
fn english(typed: &str, text: &str) -> String {
    marked(Language::English, typed, Completion::WholeWords, text)
}

#[test]
fn each_matching_word_is_marked_as_it_was_written() {
    assert_eq!(
        english("links", "Linking pages: one link, then more links."),
        "[Linking] pages: one [link], then more [links]."
    );
}

#[test]
fn a_long_text_is_cut_around_the_first_cluster_of_matches() {
    let text = "One two three four five six seven eight nine ten eleven twelve \
                thirteen fourteen fifteen sixteen seventeen eighteen nineteen twenty \
                twentyone twentytwo twentythree twentyfour twentyfive sitemaps help \
                crawlers find every page of a site, and a sitemap lists them all \
                for the crawler to read in one go, which saves time on large sites \
                with many thousands of posts in many languages and folders.";
    assert_eq!(
        english("sitemap", text),
        "\u{2026} twentythree twentyfour twentyfive [sitemaps] help crawlers find every page \
         of a site, and a [sitemap] lists them all for the crawler to read in one go, which \
         saves time on \u{2026}"
    );
}

#[test]
fn a_text_with_no_match_shows_its_opening_words_unmarked() {
    assert_eq!(
        english("robots", "Anchor text tells readers where a link goes."),
        "Anchor text tells readers where a link goes."
    );
    let long = "word ".repeat(40);
    let opening = format!("{}\u{2026}", "word ".repeat(30));
    assert_eq!(english("robots", &long), opening);
}

#[test]
fn the_word_being_typed_is_marked_from_its_start() {
    let typed = marked(
        Language::English,
        "site",
        Completion::LastWordStarts,
        "Sitemaps and site speed.",
    );
    assert_eq!(typed, "[Sitemaps] and [site] speed.");
}

#[test]
fn german_words_are_marked_by_their_stem() {
    let text = "Viele Häuser, das Haus.";
    let found = marked(Language::German, "Haus", Completion::WholeWords, text);
    assert_eq!(found, "Viele [Häuser], das [Haus].");
}

#[test]
fn an_empty_text_has_no_parts() {
    let analyzer = Analyzer::new(Language::English);
    let query = Query::parse(&analyzer, "links", Completion::WholeWords).unwrap();
    assert_eq!(Snippet::of(&analyzer, &query, "").parts(), []);
}
