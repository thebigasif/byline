//! `khalid db rebuild` on the demo site: the done line of task 1.10.
//! Deleting `data/site.db` and rebuilding gives the same index, and nothing
//! is written under `content/` (ADR 0001, RULES.md BL-FS-1, BL-FS-2).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use khalid_core::{Locale, Slug};
use khalid_db::{Database, IndexedPost};
use serde_json::Value;

const DEMO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/demo-site");

/// `name` inside `dir`.
#[cfg(test)]
fn within(dir: &Path, name: &str) -> PathBuf {
    let mut path = dir.to_path_buf();
    path.push(name);
    path
}

/// Copies the folder `from` into `to`, file by file. A `data` folder is
/// left out: tests that build the demo site in place fill it with image
/// sizes, and this copy must start with none.
#[cfg(test)]
fn copy(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().into_string().unwrap();
        if name == "data" {
            continue;
        }
        if entry.file_type().unwrap().is_dir() {
            copy(&entry.path(), &within(to, &name));
        } else {
            fs::copy(entry.path(), within(to, &name)).unwrap();
        }
    }
}

/// Every file under `dir` with its bytes, in path order.
#[cfg(test)]
fn files(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut found = BTreeMap::new();
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            found.extend(files(&path));
        } else {
            found.insert(path.clone(), fs::read(&path).unwrap());
        }
    }
    found
}

#[cfg(test)]
fn khalid(site: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_khalid"))
        .arg("--site")
        .arg(site)
        .args(args)
        .output()
        .unwrap()
}

/// The index as the database holds it: every post of each language.
#[cfg(test)]
fn index(site: &Path) -> Vec<IndexedPost> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let db = Database::open(&within(site, "data/site.db")).await.unwrap();
        let main = db.site(&Slug::try_new("main").unwrap()).await.unwrap();
        let mut posts = Vec::new();
        for language in ["en", "de"] {
            let locale = Locale::try_new(language).unwrap();
            posts.extend(db.posts(main, &locale, "posts", 100).await.unwrap());
        }
        db.close().await;
        posts
    })
}

#[test]
fn deleting_the_database_and_rebuilding_gives_the_same_index() {
    let dir = tempfile::tempdir().unwrap();
    let site = within(dir.path(), "demo-site");
    copy(Path::new(DEMO), &site);
    let content = files(&within(&site, "content"));

    let output = khalid(&site, &["db", "rebuild"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let said = String::from_utf8_lossy(&output.stdout);
    assert!(said.contains("10 posts"), "{said}");
    let first = index(&site);
    assert_eq!(first.len(), 10);
    assert_eq!(first[0].slug.as_str(), "planning-a-topic-cluster");
    assert_eq!(first[0].translation.as_deref(), Some("topic-cluster"));

    for name in ["data/site.db", "data/site.db-wal", "data/site.db-shm"] {
        let _ = fs::remove_file(within(&site, name));
    }
    let output = khalid(&site, &["db", "rebuild", "--json"]);
    assert_eq!(output.status.code(), Some(0));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["posts"], 10, "{json}");
    assert_eq!(index(&site), first);
    assert_eq!(
        files(&within(&site, "content")),
        content,
        "content/ changed"
    );
}

/// Every author and category in the index, as `language.slug name`.
#[cfg(test)]
fn lists(site: &Path) -> (Vec<String>, Vec<String>) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let db = Database::open(&within(site, "data/site.db")).await.unwrap();
        let main = db.site(&Slug::try_new("main").unwrap()).await.unwrap();
        let authors = db.authors(main).await.unwrap();
        let categories = db.categories(main).await.unwrap();
        db.close().await;
        let named = |locale: &Locale, slug: &Slug, name: &str| {
            format!("{}.{} {name}", locale.as_str(), slug.as_str())
        };
        (
            authors
                .iter()
                .map(|a| named(&a.locale, &a.slug, &a.name))
                .collect(),
            categories
                .iter()
                .map(|c| named(&c.locale, &c.slug, &c.name))
                .collect(),
        )
    })
}

#[test]
fn a_rebuild_indexes_the_authors_and_the_categories() {
    let dir = tempfile::tempdir().unwrap();
    let site = within(dir.path(), "demo-site");
    copy(Path::new(DEMO), &site);
    let output = khalid(&site, &["db", "rebuild"]);
    assert_eq!(output.status.code(), Some(0));
    let (authors, categories) = lists(&site);
    assert_eq!(
        authors,
        [
            "de.asif Asif",
            "de.lena-vogt Lena Vogt",
            "en.asif Asif",
            "en.lena-vogt Lena Vogt",
        ]
    );
    assert_eq!(
        categories,
        [
            "de.on-page-seo Onpage-SEO",
            "en.on-page-seo On-page SEO",
            "de.technical-seo Technisches SEO",
            "en.technical-seo Technical SEO",
            "de.content-strategy Content-Strategie",
            "en.content-strategy Content strategy",
        ]
    );
}

#[test]
fn a_post_with_no_slug_is_explained() {
    let dir = tempfile::tempdir().unwrap();
    let site = within(dir.path(), "demo-site");
    copy(Path::new(DEMO), &site);
    fs::write(
        within(&site, "content/en/posts/2026-10-20-untitled.md"),
        "---\ntitle: A draft\ndate: 2026-10-20\n---\n\nA draft with no slug yet.\n",
    )
    .unwrap();
    let output = khalid(&site, &["db", "rebuild"]);
    assert_eq!(output.status.code(), Some(1));
    let message = String::from_utf8_lossy(&output.stderr);
    assert!(
        message.contains("2026-10-20-untitled.md has no slug"),
        "{message}"
    );
    assert_eq!(message.lines().count(), 3, "{message}");
    assert!(
        !within(&site, "data").exists(),
        "the database was made anyway"
    );
}
