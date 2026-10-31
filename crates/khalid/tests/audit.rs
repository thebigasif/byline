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
        .args(["--site", site.to_str().unwrap(), "audit", "--offline"])
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
        file(site.path(), "issues.csv").starts_with("severity,category,check,url,detail,why,fix\n")
    );
    let links = file(site.path(), "links.csv");
    assert!(
        links.contains("https://www.example.com/blog/why-internal-links-matter/,"),
        "{links}"
    );
}

#[test]
fn the_ai_search_checks_report_each_page_with_its_reason() {
    let site = demo_copy();
    let mut post = site.path().to_path_buf();
    post.push("content/en/posts/2026-09-21-how-to-write-a-title-tag.md");
    let text = fs::read_to_string(&post).unwrap();
    let short = text.replacen(
        " The headline on the page can be longer and warmer. The title tag is for the results page, where space is short and the reader is scanning.",
        "",
        1,
    );
    assert_ne!(
        text, short,
        "the answer to shorten is gone from the demo post"
    );
    fs::write(&post, short).unwrap();
    let output = audit(site.path(), &["--json"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let body: Value = serde_json::from_str(&stdout).unwrap_or_else(|_| panic!("{stdout}"));
    let capsule = body["issues"]
        .as_array()
        .unwrap()
        .iter()
        .find(|issue| issue["check"] == "answer-capsule")
        .unwrap_or_else(|| panic!("{stdout}"));
    assert_eq!(
        capsule["url"],
        "https://www.example.com/blog/how-to-write-a-title-tag/"
    );
    assert_eq!(
        capsule["detail"],
        "22 words under \"Title tag or headline?\""
    );
    assert_eq!(capsule["category"], "AI search");
    // The demo's own speed advice gives a number with no source or year.
    let number = body["issues"]
        .as_array()
        .unwrap()
        .iter()
        .find(|issue| issue["check"] == "unsourced-number")
        .unwrap_or_else(|| panic!("{stdout}"));
    assert_eq!(
        number["url"],
        "https://www.example.com/blog/fixing-slow-pages/"
    );
    assert_eq!(number["severity"], "notice");
    assert!(
        capsule["why"]
            .as_str()
            .unwrap()
            .contains("lift the paragraph")
    );
}

#[test]
fn the_audit_takes_its_limits_from_lint_toml_and_its_day_from_the_clock() {
    let site = demo_copy();
    let mut settings = site.path().to_path_buf();
    settings.push("lint.toml");
    fs::write(&settings, "[thresholds]\nstale-months = 0\n").unwrap();
    let output = audit(site.path(), &["--json"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let body: Value = serde_json::from_str(&stdout).unwrap_or_else(|_| panic!("{stdout}"));
    let checked = |check: &str| -> Vec<&str> {
        body["issues"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|issue| issue["check"] == check)
            .map(|issue| issue["url"].as_str().unwrap())
            .collect()
    };
    // With no months of grace, every post is past its review window.
    assert!(
        checked("stale-page").contains(&"https://www.example.com/blog/how-to-write-a-title-tag/"),
        "{stdout}"
    );
    // Every post shows its dates, and every answer is open.
    assert_eq!(checked("date-not-shown"), Vec::<&str>::new(), "{stdout}");
    assert_eq!(checked("hidden-answer"), Vec::<&str>::new(), "{stdout}");
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
    assert!(said.contains("A reader who follows the link lands on an error page"));
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
