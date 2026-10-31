//! `khalid audit` checks links to other sites for dead ones, through the
//! public client; `--offline` leaves them unchecked (KH-LNT-5).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

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

/// Gives the demo post a link to `href`.
#[cfg(test)]
fn link_to(site: &Path, href: &str) {
    let mut post = site.to_path_buf();
    post.push("content/en/posts/2026-09-14-why-internal-links-matter.md");
    let text = fs::read_to_string(&post).unwrap();
    let linked = text.replacen(
        "Start with one link per post.",
        &format!("Start with one link per post, as [this study]({href}) shows."),
        1,
    );
    assert_ne!(
        text, linked,
        "the sentence to change is gone from the demo post"
    );
    fs::write(&post, linked).unwrap();
}

/// Runs `khalid audit --json` with every outside host answered by the
/// stand-in at `stand_in`, and returns the dead-link issues and the exit.
#[cfg(test)]
fn dead_links(site: &Path, stand_in: &str, args: &[&str]) -> (Vec<Value>, Output) {
    let hosts = ["serpdad.com", "studies.example.org"]
        .map(|host| format!("{host}={stand_in}"))
        .join(",");
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.to_str().unwrap(), "--test-hosts", &hosts])
        .args(["audit", "--json"])
        .args(args)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let body: Value = serde_json::from_str(&stdout).unwrap_or_else(|_| panic!("{stdout}"));
    let dead = body["issues"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|issue| issue["check"] == "broken-external")
        .cloned()
        .collect();
    (dead, output)
}

#[tokio::test]
async fn a_dead_link_to_another_site_is_a_warning_on_its_page() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/gone"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    let port = server.address().port();
    let site = demo_copy();
    link_to(
        site.path(),
        &format!("http://studies.example.org:{port}/gone"),
    );
    let (dead, output) = dead_links(site.path(), "127.0.0.1", &[]);
    // The stand-in speaks plain HTTP, so the demo's HTTPS links to other
    // sites fail too; only the planted link is this test's.
    let planted: Vec<&Value> = dead
        .iter()
        .filter(|issue| {
            issue["detail"]
                .as_str()
                .unwrap()
                .starts_with("http://studies.example.org")
        })
        .collect();
    assert_eq!(planted.len(), 1, "{dead:?}");
    let issue = planted.first().unwrap();
    assert_eq!(
        issue["url"],
        "https://www.example.com/blog/why-internal-links-matter/"
    );
    assert_eq!(
        issue["detail"],
        format!("http://studies.example.org:{port}/gone (404)")
    );
    assert_eq!(issue["severity"], "warning");
    // A warning does not fail the audit.
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn offline_leaves_links_to_other_sites_unchecked() {
    let site = demo_copy();
    link_to(site.path(), "http://studies.example.org/gone");
    // The stand-in address answers nothing, so any request would fail.
    let (dead, output) = dead_links(site.path(), "127.0.0.1", &["--offline"]);
    assert_eq!(dead, Vec::<Value>::new());
    assert_eq!(output.status.code(), Some(0));
}
