//! Words and terms: each language's rules, and where each word sits.

use khalid_core::Locale;
use khalid_search::{Analyzer, Language};

fn terms(language: Language, text: &str) -> Vec<String> {
    Analyzer::new(language)
        .terms(text)
        .map(|term| term.as_str().to_owned())
        .collect()
}

#[test]
fn each_locale_picks_its_language_by_its_first_part() {
    let of = |tag: &str| Language::of(&Locale::try_new(tag).unwrap());
    assert_eq!(of("en"), Language::English);
    assert_eq!(of("en-GB"), Language::English);
    assert_eq!(of("de-AT"), Language::German);
    assert_eq!(of("fr"), Language::Other);
}

#[test]
fn english_words_meet_on_their_stem() {
    assert_eq!(
        terms(Language::English, "Linking, links and LINKED pages"),
        ["link", "link", "and", "link", "page"]
    );
    assert_eq!(
        terms(Language::English, "Google's sitemap"),
        terms(Language::English, "google sitemaps")
    );
}

#[test]
fn german_words_meet_on_their_stem_with_or_without_umlauts() {
    let houses = terms(Language::German, "Häuser");
    assert_eq!(houses, terms(Language::German, "Haus"));
    assert_eq!(
        terms(Language::German, "Verlinkungen"),
        terms(Language::German, "Verlinkung")
    );
    assert_eq!(
        terms(Language::German, "Straße"),
        terms(Language::German, "Strasse")
    );
}

#[test]
fn other_languages_match_whole_words_in_lower_case() {
    assert_eq!(
        terms(Language::Other, "Liens internes"),
        ["liens", "internes"]
    );
}

#[test]
fn each_word_keeps_its_bytes_in_the_text() {
    let text = "Größe, und Maß.";
    let analyzer = Analyzer::new(Language::German);
    let words: Vec<&str> = analyzer
        .words(text)
        .filter_map(|word| text.get(word.range))
        .collect();
    assert_eq!(words, ["Größe", "und", "Maß"]);
}

#[test]
fn an_apostrophe_stays_only_inside_a_word() {
    let text = "'Don't' say it\u{2019}s 'quoted'";
    let analyzer = Analyzer::new(Language::Other);
    let words: Vec<&str> = analyzer
        .words(text)
        .filter_map(|word| text.get(word.range))
        .collect();
    assert_eq!(words, ["Don't", "say", "it\u{2019}s", "quoted"]);
    assert_eq!(terms(Language::Other, "Don't it\u{2019}s"), ["dont", "its"]);
}

#[test]
fn numbers_are_words_and_a_word_too_long_to_be_real_is_left_out() {
    let hash = "a".repeat(61);
    let text = format!("Khalid 2026 {hash} end");
    assert_eq!(terms(Language::English, &text), ["khalid", "2026", "end"]);
}
