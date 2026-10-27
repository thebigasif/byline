//! What the index stores for a page, and what a query looks up.

use khalid_search::{Analyzer, Completion, FieldCounts, Language, Query};

#[test]
fn a_page_is_indexed_with_each_term_counted_per_field() {
    let analyzer = Analyzer::new(Language::English);
    let text = analyzer.index("Internal links", "Why links matter", "Link your posts.");
    assert_eq!(text.lengths(), FieldCounts::new(2, 3, 3));
    let link = text
        .terms()
        .iter()
        .find(|(term, _)| term.as_str() == "link");
    assert_eq!(
        link.map(|(_, counts)| *counts),
        Some(FieldCounts::new(1, 1, 1))
    );
}

#[test]
fn a_query_keeps_each_term_once_and_at_most_eight() {
    let analyzer = Analyzer::new(Language::English);
    let query = Query::parse(
        &analyzer,
        "links Linking a b c d e f g h",
        Completion::WholeWords,
    );
    let terms: Vec<String> = query
        .unwrap()
        .terms()
        .iter()
        .map(|term| term.term().as_str().to_owned())
        .collect();
    assert_eq!(terms, ["link", "a", "b", "c", "d", "e", "f", "g"]);
    assert!(Query::parse(&analyzer, " ,. ", Completion::WholeWords).is_none());
}

#[test]
fn only_the_last_word_while_typing_matches_the_start_of_a_term() {
    let analyzer = Analyzer::new(Language::English);
    let typing = Query::parse(&analyzer, "Sit", Completion::LastWordStarts).unwrap();
    let [word] = typing.terms() else {
        panic!("one word: {typing:?}");
    };
    assert!(word.matches("sitemap"));
    assert!(word.matches("sit"));
    assert!(!word.matches("speed"));
    let submitted = Query::parse(&analyzer, "Sit", Completion::WholeWords).unwrap();
    let [word] = submitted.terms() else {
        panic!("one word: {submitted:?}");
    };
    assert_eq!(word.start(), None);
    assert!(!word.matches("sitemap"));
}

#[test]
fn a_last_word_that_repeats_an_earlier_one_is_the_one_still_typed() {
    let analyzer = Analyzer::new(Language::English);
    let query = Query::parse(&analyzer, "Link sitemap Lin", Completion::LastWordStarts).unwrap();
    let starts: Vec<Option<&str>> = query.terms().iter().map(|term| term.start()).collect();
    assert_eq!(starts, [None, None, Some("lin")]);
    let query = Query::parse(&analyzer, "link sitemap link", Completion::LastWordStarts).unwrap();
    let starts: Vec<Option<&str>> = query.terms().iter().map(|term| term.start()).collect();
    assert_eq!(starts, [Some("link"), None]);
}
