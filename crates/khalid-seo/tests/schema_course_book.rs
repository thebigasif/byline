//! The course and book blocks: `Course` and `Book` (brief G15, KH-SEO-3).
//! Google shows no rich result for either, so the check raises nothing.

#![cfg(test)]

mod common;

use common::{demo_site, internal_links_post, internal_links_post_de, locale, url};
use khalid_seo::{Book, Course, SchemaBlock, SchemaGraph, SeoPage, check_graph, page_graph};
use serde_json::{Value, json};

const PAGE: &str = "https://www.example.com/blog/why-internal-links-matter/";

/// The graph of `post` with `block`, and its last node as JSON.
fn with_block(mut post: SeoPage, block: SchemaBlock) -> (SchemaGraph, Value) {
    post.blocks = vec![block];
    let graph = page_graph(&demo_site(), &post).unwrap();
    let last = graph.nodes().last().unwrap().clone();
    let mut json: Value = serde_json::from_str(&SchemaGraph::new(vec![last]).to_json()).unwrap();
    (graph, json["@graph"][0].take())
}

/// Every issue the check raises about nodes of `kind`.
fn issues_about(graph: &SchemaGraph, kind: &str) -> Vec<String> {
    let issues = check_graph(graph).into_iter();
    let found = issues.filter(|issue| issue.kind() == kind);
    found.map(|issue| issue.property().to_owned()).collect()
}

#[test]
fn a_course_is_provided_by_the_site_publisher() {
    let course = Course::new(
        "Internal links for beginners",
        "Five short lessons on adding internal links to an older blog.",
    );
    let (graph, node) = with_block(internal_links_post(), SchemaBlock::Course(course));
    let want = json!({
        "@type": "Course",
        "@id": format!("{PAGE}#course-1"),
        "name": "Internal links for beginners",
        "description": "Five short lessons on adding internal links to an older blog.",
        "provider": {"@id": "https://www.example.com/blog/#organization"}
    });
    assert_eq!(node, want);
    assert_eq!(issues_about(&graph, "Course"), Vec::<String>::new());
}

#[test]
fn a_book_names_its_author_and_what_it_was_given() {
    let mut book = Book::new("The internal links handbook", "Asif");
    book.language = Some(locale("en"));
    book.isbn = Some(String::from("978-0-00-000000-2"));
    book.url = Some(url("https://www.example.com/blog/handbook/"));
    let (graph, node) = with_block(internal_links_post_de(), SchemaBlock::Book(book));
    assert_eq!(node["@type"], "Book");
    assert_eq!(node["author"], json!({"@type": "Person", "name": "Asif"}));
    assert_eq!(
        node["inLanguage"], "en",
        "the book's language, not the page's"
    );
    assert_eq!(node["isbn"], "978-0-00-000000-2");
    assert_eq!(node["url"], "https://www.example.com/blog/handbook/");
    assert_eq!(issues_about(&graph, "Book"), Vec::<String>::new());

    let (_, bare) = with_block(
        internal_links_post(),
        SchemaBlock::Book(Book::new("Notes", "Asif")),
    );
    let keys: Vec<&String> = bare.as_object().unwrap().keys().collect();
    assert_eq!(keys, ["@type", "@id", "name", "author"]);
}
