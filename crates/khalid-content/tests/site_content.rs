//! Every content type is read with its own schema, and the demo site reads
//! with no errors (KH-CON-1).

use std::fs;
use std::path::Path;

use khalid_content::{ContentTree, read_content};

const DEMO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/demo-site");

#[test]
fn the_demo_site_reads_with_every_type_typed() {
    let content = read_content(&ContentTree::walk(Path::new(DEMO)).unwrap()).unwrap();
    assert_eq!(
        content.posts.len(),
        10,
        "five posts in each of two languages"
    );
    assert_eq!(content.pages.len(), 2);
    assert_eq!(content.authors.len(), 4);
    assert_eq!(content.glossary.len(), 2);
    assert_eq!(content.case_studies.len(), 1);
    assert_eq!(content.changelog.len(), 1);
    assert_eq!(content.snippets.len(), 1);
    assert_eq!(content.indexes.len(), 1);

    let study = &content.case_studies[0].front;
    assert_eq!(study.client.as_deref(), Some("Northfield Bakery"));
    assert_eq!(
        study.date.map(|d| d.to_string()).as_deref(),
        Some("2026-10-20")
    );
    let asif = content
        .authors
        .iter()
        .find(|a| a.locale.as_str() == "en" && a.front.name.as_deref() == Some("Asif"))
        .unwrap();
    assert_eq!(asif.front.same_as[0].as_str(), "https://serpdad.com/");
    assert_eq!(
        content.changelog[0].front.version.as_deref(),
        Some("2026.10")
    );
    assert_eq!(
        content.indexes[0].front.title.as_deref(),
        Some("Field notes on SEO")
    );
    assert!(content.glossary.iter().all(|g| g.front.term.is_some()));
    assert_eq!(
        content.posts[0].locale.as_str(),
        "de",
        "path order puts de first"
    );
}

/// A site with one file at `path` holding `text`.
#[cfg(test)]
fn site_with(path: &str, text: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let mut full = dir.path().to_path_buf();
    full.push(path);
    fs::create_dir_all(full.parent().unwrap()).unwrap();
    fs::write(full, text).unwrap();
    dir
}

#[test]
fn each_type_refuses_fields_from_another_type() {
    let cases = [
        (
            "content/en/authors/asif.md",
            "---\ntitle: Asif\n---\n",
            "title",
        ),
        (
            "content/en/glossary/seo.md",
            "---\nname: SEO\n---\n",
            "name",
        ),
        (
            "content/en/changelog/2026-10.md",
            "---\nclient: Acme\n---\n",
            "client",
        ),
        (
            "content/en/posts/_index.md",
            "---\ndate: 2026-10-26\n---\n",
            "date",
        ),
    ];
    for (path, text, field) in cases {
        let dir = site_with(path, text);
        let error = read_content(&ContentTree::walk(dir.path()).unwrap())
            .unwrap_err()
            .to_string();
        assert!(
            error.starts_with(&format!("{path}, line 2, field \"{field}\": unknown field")),
            "{error}"
        );
    }
}

#[test]
fn a_collection_with_any_other_name_holds_posts() {
    let dir = site_with(
        "content/en/guides/choosing-a-crm.md",
        "---\ntitle: Choosing a CRM\nauthor: asif\n---\n",
    );
    let content = read_content(&ContentTree::walk(dir.path()).unwrap()).unwrap();
    assert_eq!(content.posts.len(), 1);
    assert_eq!(
        content.posts[0]
            .front
            .author
            .as_ref()
            .map(khalid_core::Slug::as_str),
        Some("asif")
    );
}

#[test]
fn an_author_profile_must_be_a_web_address() {
    let dir = site_with(
        "content/en/authors/asif.md",
        "---\nsame_as:\n  - serpdad.com\n---\n",
    );
    let error = read_content(&ContentTree::walk(dir.path()).unwrap())
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("field \"same_as\"") || error.contains("line 3"),
        "{error}"
    );
    assert!(error.contains("not a full URL"), "{error}");
}
