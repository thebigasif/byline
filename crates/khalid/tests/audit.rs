//! `khalid audit` on the demo site, end to end: build, crawl, score, the
//! files it keeps and the trend (KH-LNT-5).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;

/// A copy of the demo site, so the audit's `data/` stays out of the
/// fixture and a test can change a post.
#[cfg(test)]
fn demo_copy() -> TempDir {
    let mut demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    demo.push("../../fixtures/demo-site");
    let temp = tempfile::tempdir().unwrap();
    copy_dir(&demo, temp.path());
    temp
}

#[cfg(test)]
fn copy_dir(from: &Path, into: &Path) {
    fs::create_dir_all(into).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        if entry.file_name() == "data" {
            continue;
        }
        let mut target = into.to_path_buf();
        target.push(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

#[cfg(test)]
fn audit(site: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.to_str().unwrap(), "audit"])
        .args(args)
        .output()
        .unwrap()
}

#[cfg(test)]
fn file(site: &Path, name: &str) -> String {
    let mut path = site.to_path_buf();
    path.push("data/audit");
    path.push(name);
    fs::read_to_string(path).unwrap()
}

/// The warnings the demo site has today, each a gap in the default pages
/// that another track's brief fixes. A fix leaves this test green; a new
/// kind of warning fails it.
const KNOWN_WARNINGS: [&str; 3] = [
    // The home, list, author, glossary and HTML sitemap pages have no
    // meta description.
    "description-missing",
    // The German home and author pages reuse the English titles.
    "title-duplicate",
    // No page links to the HTML sitemap page.
    "orphan",
];

#[test]
fn the_demo_site_scores_100_with_no_errors_and_only_known_warnings() {
    let site = demo_copy();
    let output = audit(site.path(), &["--json"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let body: Value = serde_json::from_str(&stdout).unwrap_or_else(|_| panic!("{stdout}"));
    let problems: Vec<&Value> = body["issues"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|issue| issue["severity"] == "error")
        .chain(body["issues"].as_array().unwrap().iter().filter(|issue| {
            issue["severity"] == "warning"
                && !KNOWN_WARNINGS.contains(&issue["check"].as_str().unwrap())
        }))
        .collect();
    assert_eq!(problems, Vec::<&Value>::new(), "{stdout}");
    assert_eq!(body["score"], 100, "{stdout}");
    assert_eq!(body["errors"], 0);
    assert!(body["pages"].as_u64().unwrap() > 10, "{stdout}");
    assert_eq!(output.status.code(), Some(0));
    assert!(
        file(site.path(), "issues.csv").starts_with("severity,category,check,url,detail,fix\n")
    );
    let links = file(site.path(), "links.csv");
    assert!(
        links.contains("https://www.example.com/blog/why-internal-links-matter/,"),
        "{links}"
    );
}

#[test]
fn each_run_adds_to_the_trend() {
    let site = demo_copy();
    assert_eq!(audit(site.path(), &[]).status.code(), Some(0));
    let output = audit(site.path(), &[]);
    let said = String::from_utf8_lossy(&output.stdout);
    assert!(said.contains("Health score: 100 out of 100."), "{said}");
    assert!(
        said.contains("The last scores, oldest first: 100, 100."),
        "{said}"
    );
    assert_eq!(file(site.path(), "runs.jsonl").lines().count(), 2);
}

#[test]
fn a_link_to_a_missing_page_fails_the_audit_and_says_how_to_fix_it() {
    let site = demo_copy();
    let mut post = site.path().to_path_buf();
    post.push("content/en/posts/2026-09-14-why-internal-links-matter.md");
    let text = fs::read_to_string(&post).unwrap();
    let broken = text.replacen(
        "Start with one link per post.",
        "Start with one link per post, such as [our old guide](/blog/old-guide/).",
        1,
    );
    assert_ne!(
        text, broken,
        "the sentence to change is gone from the demo post"
    );
    fs::write(&post, broken).unwrap();
    let output = audit(site.path(), &[]);
    let said = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(1), "{said}");
    assert!(said.contains("Page has links to broken page"), "{said}");
    assert!(said.contains("Change the link to a page that exists, or remove it."));
    let issues = file(site.path(), "issues.csv");
    assert!(
        issues.contains("https://www.example.com/blog/old-guide/"),
        "{issues}"
    );
}

#[test]
fn audit_needs_a_site_folder() {
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .arg("audit")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let said = String::from_utf8_lossy(&output.stderr);
    assert!(said.contains("needs a site folder"), "{said}");
}
