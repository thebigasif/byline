//! `khalid new --branch`: in git mode each draft is a branch named after
//! the post, with the new post committed on it, so publishing is the
//! merge (KH-EDT-4, plan section 4.6).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;

/// Who the test's commits are by.
const WHO: [(&str, &str); 4] = [
    ("GIT_AUTHOR_NAME", "Writer"),
    ("GIT_AUTHOR_EMAIL", "writer@example.com"),
    ("GIT_COMMITTER_NAME", "Writer"),
    ("GIT_COMMITTER_EMAIL", "writer@example.com"),
];

#[cfg(test)]
fn git(folder: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(folder)
        .args(args)
        .envs(WHO)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        text(&output.stderr)
    );
    text(&output.stdout).trim().to_owned()
}

#[cfg(test)]
fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// `relative` inside `folder`.
#[cfg(test)]
fn within(folder: &Path, relative: &str) -> PathBuf {
    let mut path = folder.to_path_buf();
    path.push(relative);
    path
}

/// A copy of the demo site.
#[cfg(test)]
fn demo() -> TempDir {
    let demo = within(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        "../../fixtures/demo-site/.",
    );
    let temp = tempfile::tempdir().unwrap();
    let copied = Command::new("cp")
        .arg("-R")
        .args([&demo, &temp.path().to_path_buf()])
        .status();
    assert!(copied.unwrap().success());
    temp
}

/// A copy of the demo site in a repository on `main` with one commit.
#[cfg(test)]
fn site() -> TempDir {
    let temp = demo();
    fs::write(within(temp.path(), ".gitignore"), "data/\npublic/\n").unwrap();
    git(temp.path(), &["init", "--quiet", "-b", "main"]);
    git(temp.path(), &["add", "."]);
    git(temp.path(), &["commit", "--quiet", "-m", "The demo site"]);
    temp
}

#[cfg(test)]
fn new_post(site: &Path, title: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args([
            "--site",
            site.to_str().unwrap(),
            "--json",
            "new",
            title,
            "--branch",
        ])
        .envs(WHO)
        .output()
        .unwrap()
}

#[test]
fn a_new_post_is_committed_on_its_own_branch() {
    let site = site();
    let made = new_post(site.path(), "Choosing anchor text");
    assert_eq!(made.status.code(), Some(0), "{}", text(&made.stderr));
    let body: Value = serde_json::from_slice(&made.stdout).unwrap();
    assert_eq!(body["branch"], "post/choosing-anchor-text", "{body}");
    let path = body["path"].as_str().unwrap();
    assert!(path.ends_with("-choosing-anchor-text.md"), "{body}");
    assert_eq!(
        git(site.path(), &["branch", "--show-current"]),
        "post/choosing-anchor-text"
    );
    assert_eq!(
        git(site.path(), &["log", "-1", "--format=%s"]),
        "Draft: Choosing anchor text"
    );
    assert_eq!(
        git(site.path(), &["status", "--porcelain"]),
        "",
        "the post is committed"
    );
    let on_main = git(site.path(), &["ls-tree", "-r", "--name-only", "main"]);
    assert!(
        !on_main.contains("choosing-anchor-text"),
        "main is untouched"
    );
}

#[test]
fn a_branch_that_exists_is_refused_and_nothing_changes() {
    let site = site();
    let made = new_post(site.path(), "Choosing anchor text");
    assert_eq!(made.status.code(), Some(0), "{}", text(&made.stderr));
    git(site.path(), &["switch", "--quiet", "main"]);
    let again = new_post(site.path(), "Choosing anchor text");
    assert_eq!(again.status.code(), Some(1));
    let said: Value = serde_json::from_slice(&[again.stdout, again.stderr].concat()).unwrap();
    let said = said.to_string();
    assert!(said.contains("post/choosing-anchor-text"), "{said}");
    assert_eq!(git(site.path(), &["branch", "--show-current"]), "main");
    assert_eq!(git(site.path(), &["status", "--porcelain"]), "");
}

#[test]
fn outside_a_repository_it_says_how_to_start_one() {
    let site = demo();
    let made = new_post(site.path(), "Choosing anchor text");
    assert_eq!(made.status.code(), Some(1));
    let said = text(&[made.stdout, made.stderr].concat());
    assert!(said.contains("git init"), "{said}");
    let posts = fs::read_dir(within(site.path(), "content/en/posts")).unwrap();
    assert!(
        posts
            .flatten()
            .all(|f| !f.file_name().to_string_lossy().contains("anchor"))
    );
}
