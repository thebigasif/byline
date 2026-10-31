//! `khalid pull`: in git mode, the site's content repository brought up to
//! date, checked, and for a static site built again (plan section 4.1,
//! KH-OPS-2). Each test makes a bare repository, a writer's clone holding
//! the demo site, and the blog's own clone.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;

/// The demo post each test changes.
const POST: &str = "content/en/posts/2026-09-21-how-to-write-a-title-tag.md";

/// A bare repository, a writer's clone holding the demo site with `mode`,
/// and the blog's clone of it, in that order.
struct Repos {
    _origin: TempDir,
    writer: TempDir,
    blog: TempDir,
}

#[cfg(test)]
fn git(folder: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=Writer",
            "-c",
            "user.email=writer@example.com",
        ])
        .arg("-C")
        .arg(folder)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        text(&output.stderr)
    );
}

#[cfg(test)]
fn repos(mode: &str) -> Repos {
    let origin = tempfile::tempdir().unwrap();
    git(origin.path(), &["init", "--bare", "--quiet", "-b", "main"]);
    let writer = demo_copy();
    let settings = within(writer.path(), "site.toml");
    let text = fs::read_to_string(&settings).unwrap();
    fs::write(
        &settings,
        text.replace("mode = \"server\"", &format!("mode = \"{mode}\"")),
    )
    .unwrap();
    fs::write(within(writer.path(), ".gitignore"), "data/\npublic/\n").unwrap();
    let origin_path = origin.path().to_str().unwrap();
    git(writer.path(), &["init", "--quiet", "-b", "main"]);
    git(writer.path(), &["add", "."]);
    git(writer.path(), &["commit", "--quiet", "-m", "The demo site"]);
    git(writer.path(), &["remote", "add", "origin", origin_path]);
    git(writer.path(), &["push", "--quiet", "-u", "origin", "main"]);
    let blog = tempfile::tempdir().unwrap();
    git(blog.path(), &["clone", "--quiet", origin_path, "."]);
    Repos {
        _origin: origin,
        writer,
        blog,
    }
}

/// Adds a paragraph to the demo post in the writer's clone and pushes it.
#[cfg(test)]
fn push_an_edit(writer: &Path) {
    let post = within(writer, POST);
    let text = fs::read_to_string(&post).unwrap();
    fs::write(&post, format!("{text}\nOne more paragraph.\n")).unwrap();
    git(writer, &["commit", "--quiet", "-am", "Add a paragraph"]);
    git(writer, &["push", "--quiet"]);
}

/// A copy of the demo site's settings and content.
#[cfg(test)]
fn demo_copy() -> TempDir {
    let temp = tempfile::tempdir().unwrap();
    let demo = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/demo-site");
    for name in ["site.toml", "categories.toml", "README.md", "content"] {
        copy(&within(Path::new(demo), name), &within(temp.path(), name));
    }
    temp
}

#[cfg(test)]
fn copy(from: &Path, into: &Path) {
    if from.is_dir() {
        fs::create_dir_all(into).unwrap();
        for entry in fs::read_dir(from).unwrap() {
            let name = entry.unwrap().file_name();
            let name = name.to_str().unwrap();
            copy(&within(from, name), &within(into, name));
        }
    } else {
        fs::copy(from, into).unwrap();
    }
}

#[cfg(test)]
fn within(dir: &Path, name: &str) -> PathBuf {
    let mut path = dir.to_path_buf();
    path.push(name);
    path
}

#[cfg(test)]
fn pull(site: &Path, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_khalid"))
        .arg("--site")
        .arg(site)
        .arg("pull")
        .args(extra)
        .output()
        .unwrap()
}

#[cfg(test)]
fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn pull_brings_in_what_was_pushed() {
    let repos = repos("server");
    push_an_edit(repos.writer.path());
    let output = pull(repos.blog.path(), &[]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let said = text(&output.stdout);
    assert!(said.starts_with("Pulled 1 changed file"), "{said}");
    assert!(said.contains("khalid serve"), "{said}");
    let pulled = fs::read_to_string(within(repos.blog.path(), POST)).unwrap();
    assert!(pulled.ends_with("One more paragraph.\n"), "{pulled}");
    assert!(!within(repos.blog.path(), "public").exists());
}

#[test]
fn pull_with_json_names_the_commits_and_the_changed_files() {
    let repos = repos("server");
    push_an_edit(repos.writer.path());
    let output = pull(repos.blog.path(), &["--json"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["changed"], true, "{json}");
    assert_eq!(json["files"], 1, "{json}");
    assert_ne!(json["from"], json["to"], "{json}");
    assert_eq!(json["to"].as_str().unwrap().len(), 40, "{json}");
}

#[test]
fn pull_with_nothing_new_changes_nothing() {
    let repos = repos("server");
    let output = pull(repos.blog.path(), &["--json"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["changed"], false, "{json}");
    assert_eq!(json["files"], 0, "{json}");
}

#[test]
fn a_static_site_is_built_again_after_a_pull() {
    let repos = repos("static");
    push_an_edit(repos.writer.path());
    let output = pull(repos.blog.path(), &["--json"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json["pages"].as_u64().unwrap() > 0, "{json}");
    let page = within(
        repos.blog.path(),
        "public/how-to-write-a-title-tag/index.html",
    );
    let html = fs::read_to_string(&page).unwrap_or_default();
    assert!(html.contains("One more paragraph."), "{}", page.display());
}

#[test]
fn a_site_outside_git_is_explained() {
    let site = demo_copy();
    let output = pull(site.path(), &[]);
    assert_eq!(output.status.code(), Some(1));
    let message = text(&output.stderr);
    let lines: Vec<&str> = message.lines().collect();
    assert_eq!(lines.len(), 3, "{message}");
    assert!(lines[0].contains("not in a git repository"), "{message}");
}

#[test]
fn a_blog_with_its_own_commits_is_not_merged() {
    let repos = repos("server");
    push_an_edit(repos.writer.path());
    let readme = within(repos.blog.path(), "README.md");
    fs::write(&readme, "Changed on the server.\n").unwrap();
    git(
        repos.blog.path(),
        &["commit", "--quiet", "-am", "Edit on the server"],
    );
    let output = pull(repos.blog.path(), &[]);
    assert_eq!(output.status.code(), Some(1));
    let message = text(&output.stderr);
    assert_eq!(message.lines().count(), 3, "{message}");
    assert!(message.contains("git -C"), "{message}");
    let post = fs::read_to_string(within(repos.blog.path(), POST)).unwrap();
    assert!(!post.contains("One more paragraph."));
}
