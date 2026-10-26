//! The glossary from A to Z, and the search page that works with scripts off.

use khalid_render::{
    GlossaryGroup, GlossaryTerm, PageBody, PageKind, RenderGlossary, RenderSearch, Renderer,
    SearchResult,
};

use crate::common::{demo_site, meta};
use crate::html_check::assert_valid_page;
use crate::listing::main_of;

fn render(body: &PageBody, tag: &str) -> String {
    let meta = meta("Glossary", "/blog/glossary/", PageKind::Website);
    Renderer::new()
        .unwrap()
        .render(&demo_site(tag), &meta, body)
        .unwrap()
}

fn term(term: &str, slug: &str, summary: &str) -> GlossaryTerm {
    GlossaryTerm {
        term: term.to_owned(),
        href: format!("/blog/glossary/{slug}/"),
        summary: summary.to_owned(),
    }
}

fn glossary() -> RenderGlossary {
    RenderGlossary {
        title: "Glossary".to_owned(),
        intro: None,
        groups: vec![
            GlossaryGroup {
                letter: "C".to_owned(),
                anchor: "c".to_owned(),
                terms: vec![term(
                    "Canonical",
                    "canonical",
                    "The one address a page should be indexed under.",
                )],
            },
            GlossaryGroup {
                letter: "T".to_owned(),
                anchor: "t".to_owned(),
                terms: vec![term(
                    "Topical authority",
                    "topical-authority",
                    "How much a search engine trusts a site on one subject.",
                )],
            },
        ],
    }
}

fn search(query: Option<&str>, results: Vec<SearchResult>) -> PageBody {
    PageBody::Search(RenderSearch {
        title: "Search".to_owned(),
        action: "/blog/search/".to_owned(),
        query: query.map(str::to_owned),
        results,
    })
}

#[test]
fn glossary_page_in_english() {
    let page = render(&PageBody::Glossary(glossary()), "en");
    assert_valid_page(&page);
    insta::assert_snapshot!(main_of(&page));
}

#[test]
fn an_empty_glossary_says_so() {
    let mut empty = glossary();
    empty.groups.clear();
    let page = render(&PageBody::Glossary(empty), "de");
    assert_valid_page(&page);
    assert!(page.contains("<p>Das Glossar hat noch keine Begriffe.</p>"));
}

#[test]
fn search_page_before_a_query_is_only_the_form() {
    let page = render(&search(None, Vec::new()), "en");
    assert_valid_page(&page);
    assert!(
        page.contains(r#"<form class="search" role="search" action="/blog/search/" method="get">"#)
    );
    assert!(page.contains(r#"<label for="search-query">Search this blog</label>"#));
    assert!(page.contains(r#"name="q" value="""#));
    assert!(!page.contains("search-results"));
}

#[test]
fn search_results_in_english() {
    let found = SearchResult {
        title: "What a sitemap does, and what it does not".to_owned(),
        href: "/blog/what-a-sitemap-does/".to_owned(),
        excerpt: "A sitemap lists the pages you want search engines to know about.".to_owned(),
    };
    let page = render(&search(Some("sitemap"), vec![found]), "en");
    assert_valid_page(&page);
    insta::assert_snapshot!(main_of(&page));
}

#[test]
fn a_hostile_query_is_escaped_and_nothing_matched_is_explained() {
    let page = render(&search(Some(r#""><script>x</script>"#), Vec::new()), "de");
    assert_valid_page(&page);
    assert!(page.contains(r#"value="&quot;&gt;&lt;script&gt;x&lt;/script&gt;""#));
    assert!(page.contains("Ergebnisse für „&quot;&gt;&lt;script&gt;x&lt;/script&gt;“"));
    assert!(page.contains("Nichts gefunden."));
}
