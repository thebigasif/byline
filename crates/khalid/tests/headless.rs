//! A headless site (`mode = "headless"`, KH-INS-5): `khalid build` writes
//! the feeds, sitemaps and redirects of the demo site and no HTML. The
//! theme is never read, so a broken one does not stop it.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

/// The old address the demo's sitemap post lists in its `aliases`.
const ALIAS: &str = "/blog/sitemaps-explained/";

#[cfg(test)]
fn at(base: &Path, rel: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    path.push(rel);
    path
}

/// A copy of the demo site set to headless, with an alias on one post and
/// theme tokens no theme could read.
#[cfg(test)]
fn headless_demo() -> TempDir {
    let demo = at(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        "../../fixtures/demo-site",
    );
    let temp = tempfile::tempdir().unwrap();
    copy_dir(&demo, temp.path());
    let settings = at(temp.path(), "site.toml");
    let text = fs::read_to_string(&settings).unwrap();
    fs::write(
        &settings,
        text.replace("mode = \"server\"", "mode = \"headless\""),
    )
    .unwrap();
    let post = at(
        temp.path(),
        "content/en/posts/2026-09-28-what-a-sitemap-does.md",
    );
    let text = fs::read_to_string(&post).unwrap();
    let aliased = text.replace(
        "translation: sitemap\n",
        &format!("translation: sitemap\naliases:\n  - {ALIAS}\n"),
    );
    fs::write(&post, aliased).unwrap();
    let theme = at(temp.path(), "theme");
    fs::create_dir_all(&theme).unwrap();
    fs::write(at(&theme, "tokens.toml"), "[colors]\nlink = \"red-ish\"\n").unwrap();
    temp
}

#[cfg(test)]
fn copy_dir(from: &Path, into: &Path) {
    fs::create_dir_all(into).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = at(into, entry.file_name().to_str().unwrap());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// Every file under `folder`, relative to it, sorted.
#[cfg(test)]
fn files(folder: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut open = vec![folder.to_path_buf()];
    while let Some(dir) = open.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                open.push(path);
            } else {
                let rel = path.strip_prefix(folder).unwrap();
                found.push(rel.to_str().unwrap().to_owned());
            }
        }
    }
    found.sort();
    found
}

#[test]
fn a_headless_build_writes_the_feeds_sitemaps_and_redirects_and_no_html() {
    let site = headless_demo();
    let out = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .arg("--site")
        .arg(site.path())
        .arg("build")
        .arg("--out")
        .arg(out.path())
        .output()
        .unwrap();
    let said = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(said.contains("no pages"), "{said}");

    let written = files(out.path());
    let html: Vec<&String> = written
        .iter()
        .filter(|f| {
            Path::new(f)
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("html"))
        })
        .collect();
    assert!(html.is_empty(), "{html:?}");
    assert!(
        !written.iter().any(|f| f.starts_with("assets")),
        "{written:?}"
    );
    for file in [
        "sitemap.xml",
        "robots.txt",
        "llms.txt",
        "feed.xml",
        "atom.xml",
        "feed.json",
        "de/feed.xml",
        "_redirects",
    ] {
        assert!(written.iter().any(|f| f == file), "{file} in {written:?}");
    }
    let sitemaps: String = written
        .iter()
        .filter(|f| f.starts_with("sitemap"))
        .map(|f| fs::read_to_string(at(out.path(), f)).unwrap())
        .collect();
    assert!(
        sitemaps.contains("https://www.example.com/blog/what-a-sitemap-does/"),
        "{sitemaps}"
    );
    let redirects = fs::read_to_string(at(out.path(), "_redirects")).unwrap();
    assert!(
        redirects.contains(&format!("{ALIAS} /blog/what-a-sitemap-does/ 301")),
        "{redirects}"
    );
}
