//! `khalid build` writes the files that must sit at fixed places: the
//! `IndexNow` key file in the blog's folder, where `keyLocation` points
//! (KH-SEO-11, ADR "root files go in their own folder").

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

/// A key made up for these tests. An `IndexNow` key is public by design.
const KEY: &str = "5d1e7a0c9b3f4e28";

#[cfg(test)]
fn at(base: &Path, rel: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    path.push(rel);
    path
}

/// A site mounted at `mount`, with `extra` added to its settings and one
/// post that the publish gate lets through.
#[cfg(test)]
fn site(mount: &str, extra: &str) -> TempDir {
    let temp = tempfile::tempdir().unwrap();
    let settings = format!(
        "name = \"Field Notes on SEO\"\naddress = \"https://www.example.com\"\nmount = \"{mount}\"\n\
         mode = \"static\"\ndefault_language = \"en\"\nlanguages = [\"en\"]\n{extra}"
    );
    fs::write(at(temp.path(), "site.toml"), settings).unwrap();
    let folder = at(temp.path(), "content/en/posts");
    fs::create_dir_all(&folder).unwrap();
    let post = "---\ntitle: Why links matter\nslug: why-links-matter\ndate: 2026-10-01\n\
                author: asif\n---\n\nA post that links to [the home page](/).\n";
    fs::write(at(&folder, "why-links-matter.md"), post).unwrap();
    temp
}

#[cfg(test)]
fn build(site: &TempDir) -> Output {
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .arg("--site")
        .arg(site.path())
        .arg("build")
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

#[test]
fn the_indexnow_key_file_sits_in_the_blogs_folder() {
    let site = site("/blog/", &format!("\n[seo.indexnow]\nkey = \"{KEY}\"\n"));
    build(&site);
    let key_file = fs::read_to_string(at(site.path(), &format!("public/{KEY}.txt"))).unwrap();
    insta::assert_snapshot!(key_file);
}

#[test]
fn a_site_without_indexnow_has_no_key_file() {
    let site = site("/blog/", "");
    build(&site);
    assert!(at(site.path(), "public/robots.txt").is_file());
    assert!(!at(site.path(), &format!("public/{KEY}.txt")).exists());
}
