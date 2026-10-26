//! The content tree: language folders, collections, `_index.md`, sorted order
//! and content hashes.

use std::fs;
use std::path::{Path, PathBuf};

use khalid_content::{ContentTree, Entry};

/// Writes `files` under a fresh site folder and returns it.
#[cfg(test)]
fn site(files: &[(&str, &[u8])]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (path, bytes) in files {
        let mut full = dir.path().to_path_buf();
        full.push(path);
        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(full, bytes).unwrap();
    }
    dir
}

#[cfg(test)]
fn walk(dir: &Path) -> ContentTree {
    ContentTree::walk(dir).unwrap()
}

#[cfg(test)]
fn paths(entries: &[Entry]) -> Vec<PathBuf> {
    entries
        .iter()
        .map(|entry| entry.path().to_path_buf())
        .collect()
}

const POST: &[u8] = b"---\ntitle: A\n---\nText.\n";

#[test]
fn walks_the_demo_site() {
    let tree = walk(Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/demo-site"
    )));
    let languages: Vec<&str> = tree
        .languages()
        .iter()
        .map(|l| l.locale().as_str())
        .collect();
    assert_eq!(languages, ["de", "en"]);
    for language in tree.languages() {
        let names: Vec<&str> = language
            .collections()
            .iter()
            .map(khalid_content::Collection::name)
            .collect();
        assert_eq!(
            names,
            ["authors", "glossary", "posts"],
            "{}",
            language.locale()
        );
        let posts = language.collection("posts").unwrap();
        assert_eq!(posts.entries().len(), 5, "{}", language.locale());
    }
}

#[test]
fn lists_entries_in_path_order_whatever_order_they_were_written() {
    let dir = site(&[
        ("content/en/posts/c.md", POST),
        ("content/en/posts/a.md", POST),
        ("content/en/posts/b.md", POST),
    ]);
    let tree = walk(dir.path());
    let posts = tree.languages()[0].collection("posts").unwrap();
    assert_eq!(
        paths(posts.entries()),
        [
            "content/en/posts/a.md",
            "content/en/posts/b.md",
            "content/en/posts/c.md"
        ]
        .map(PathBuf::from)
    );
    assert_eq!(walk(dir.path()), tree, "a second walk gives the same tree");
}

#[test]
fn a_collection_index_is_not_an_entry() {
    let dir = site(&[
        ("content/en/guides/_index.md", b"---\ntitle: Guides\n---\n"),
        ("content/en/guides/crm.md", POST),
    ]);
    let tree = walk(dir.path());
    let guides = tree.languages()[0].collection("guides").unwrap();
    assert_eq!(
        guides.index().unwrap().path(),
        Path::new("content/en/guides/_index.md")
    );
    assert_eq!(
        paths(guides.entries()),
        [PathBuf::from("content/en/guides/crm.md")]
    );
}

#[test]
fn refuses_a_language_folder_that_is_not_a_language() {
    let dir = site(&[("content/english/posts/a.md", POST)]);
    let error = ContentTree::walk(dir.path()).unwrap_err().to_string();
    assert_eq!(
        error,
        "content/english is not a language folder: the language must be a two- or three-letter code such as en or de"
    );
}

#[test]
fn refuses_a_site_without_a_content_folder() {
    let dir = site(&[("site.toml", b"")]);
    let error = ContentTree::walk(dir.path()).unwrap_err().to_string();
    assert_eq!(error, "could not read content");
}

#[test]
fn the_hash_follows_the_text() {
    let hash = |text: &[u8]| {
        let dir = site(&[("content/en/posts/a.md", text)]);
        walk(dir.path()).languages()[0]
            .collection("posts")
            .unwrap()
            .entries()[0]
            .hash()
    };
    assert_eq!(hash(POST), hash(POST));
    assert_ne!(hash(POST), hash(b"---\ntitle: B\n---\nText.\n"));
    assert_eq!(hash(POST).to_string().len(), 64, "a hex SHA-256");
}
