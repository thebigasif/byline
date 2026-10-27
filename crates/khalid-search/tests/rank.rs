//! Ranking: the title counts more than a heading, a heading more than the
//! body, a rare word more than a common one, and every word must appear.

use khalid_search::{
    Analyzer, Completion, Corpus, FieldCounts, IndexedText, Language, Posting, Query, rank,
};

/// One page of a test site: its key, title, headings and body.
type Doc = (&'static str, &'static str, &'static str, &'static str);

/// What the database does with a query: every page's terms as the index
/// holds them, and the rows each query term finds.
#[cfg(test)]
fn search(language: Language, docs: &[Doc], text: &str, completion: Completion) -> Vec<String> {
    let analyzer = Analyzer::new(language);
    let indexed: Vec<(&str, IndexedText)> = docs
        .iter()
        .map(|(key, title, headings, body)| (*key, analyzer.index(title, headings, body)))
        .collect();
    let mut total = FieldCounts::default();
    for (_, text) in &indexed {
        total = total.plus(text.lengths());
    }
    let corpus = Corpus::new(u32::try_from(docs.len()).unwrap(), total);
    let Some(query) = Query::parse(&analyzer, text, completion) else {
        return Vec::new();
    };
    let postings: Vec<Vec<Posting<String>>> = query
        .terms()
        .iter()
        .map(|wanted| {
            let mut found = Vec::new();
            for (key, text) in &indexed {
                for (term, counts) in text.terms() {
                    if wanted.matches(term.as_str()) {
                        found.push(Posting::new(key.to_string(), text.lengths(), *counts));
                    }
                }
            }
            found
        })
        .collect();
    rank(&corpus, &postings)
}

#[cfg(test)]
fn english(docs: &[Doc], text: &str) -> Vec<String> {
    search(Language::English, docs, text, Completion::WholeWords)
}

#[test]
fn a_word_in_the_title_ranks_above_the_same_word_in_the_body() {
    let docs = [
        (
            "body",
            "Crawling",
            "",
            "A sitemap helps. A sitemap lists pages.",
        ),
        ("title", "Sitemaps", "", "Lists of pages for crawlers."),
    ];
    assert_eq!(english(&docs, "sitemap"), ["title", "body"]);
}

#[test]
fn a_word_in_a_heading_ranks_above_the_same_word_in_the_body() {
    let docs = [
        ("body", "Crawling", "Basics", "Robots files and more."),
        ("heading", "Crawling", "Robots files", "And more."),
    ];
    assert_eq!(english(&docs, "robots"), ["heading", "body"]);
}

#[test]
fn every_word_of_the_query_must_appear() {
    let docs = [
        ("both", "Sitemaps and robots", "", ""),
        ("one", "Sitemaps", "", ""),
        ("none", "Anchor text", "", ""),
    ];
    assert_eq!(english(&docs, "sitemap robots"), ["both"]);
    assert_eq!(english(&docs, "sitemap unicorn"), Vec::<String>::new());
}

#[test]
fn a_rare_word_counts_more_than_a_common_one() {
    let docs = [
        ("common", "", "", "Link link link and one sitemap."),
        ("rare", "", "", "One link and a sitemap sitemap sitemap."),
        ("a", "", "", "Link"),
        ("b", "", "", "Link"),
        ("c", "", "", "Link"),
    ];
    assert_eq!(english(&docs, "link sitemap"), ["rare", "common"]);
}

#[test]
fn the_same_count_in_a_shorter_body_ranks_higher() {
    let docs = [
        (
            "long",
            "",
            "",
            "Schema helps search engines read every part of a long and winding page.",
        ),
        ("short", "", "", "Schema helps."),
    ];
    assert_eq!(english(&docs, "schema"), ["short", "long"]);
}

#[test]
fn equal_scores_keep_the_order_of_their_keys() {
    let docs = [
        ("b", "Slugs", "", ""),
        ("a", "Slugs", "", ""),
        ("c", "Slugs", "", ""),
    ];
    assert_eq!(english(&docs, "slug"), ["a", "b", "c"]);
}

#[test]
fn the_last_word_may_be_the_start_of_a_word_while_typing() {
    let docs = [("map", "Sitemaps", "", ""), ("site", "Site speed", "", "")];
    let typed = |text| search(Language::English, &docs, text, Completion::LastWordStarts);
    assert_eq!(typed("sitem"), ["map"]);
    assert_eq!(typed("sit"), ["map", "site"]);
    assert!(
        english(&docs, "sitem").is_empty(),
        "a submitted query matches whole words"
    );
}

#[test]
fn a_german_query_finds_the_stem_with_or_without_umlauts() {
    let docs = [
        ("haus", "Das Haus am See", "", ""),
        ("auto", "Autos", "", ""),
    ];
    let found = search(Language::German, &docs, "Häuser", Completion::WholeWords);
    assert_eq!(found, ["haus"]);
}
