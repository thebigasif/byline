//! `khalid export`: a static site written whole into `public/`, with
//! nothing left from earlier exports, and swapped in only once it is
//! complete (plan sections 4.1 and 7, KH-OPS-1, KH-OPS-2).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;

/// A copy of the demo site's settings and content with `mode`, and a
/// `public/` holding a page from an earlier export.
#[cfg(test)]
fn site(mode: &str) -> TempDir {
    let temp = tempfile::tempdir().unwrap();
    let demo = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/demo-site");
    for name in ["site.toml", "categories.toml", "content"] {
        copy(&within(Path::new(demo), name), &within(temp.path(), name));
    }
    let settings = within(temp.path(), "site.toml");
    let text = fs::read_to_string(&settings).unwrap();
    let text = text.replace("mode = \"server\"", &format!("mode = \"{mode}\""));
    fs::write(&settings, text).unwrap();
    fs::create_dir_all(within(temp.path(), "public/a-deleted-post")).unwrap();
    fs::write(
        within(temp.path(), "public/a-deleted-post/index.html"),
        "old",
    )
    .unwrap();
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
fn export(site: &Path, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_khalid"))
        .arg("--site")
        .arg(site)
        .arg("export")
        .args(extra)
        .output()
        .unwrap()
}

#[cfg(test)]
fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// The names of the folders an export left behind in the site, such as
/// a half-written one.
#[cfg(test)]
fn leftovers(site: &Path) -> Vec<String> {
    fs::read_dir(site)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with(".export"))
        .collect()
}

#[test]
fn export_writes_the_static_site_and_drops_pages_from_earlier_exports() {
    let site = site("static");
    let output = export(site.path(), &[]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let said = text(&output.stdout);
    assert!(said.starts_with("Exported "), "{said}");
    let page = within(site.path(), "public/how-to-write-a-title-tag/index.html");
    assert!(page.is_file(), "{said}");
    assert!(!within(site.path(), "public/a-deleted-post").exists());
    assert_eq!(leftovers(site.path()), Vec::<String>::new(),);
}

#[test]
fn export_with_json_names_the_folder_and_counts_the_pages() {
    let site = site("static");
    let output = export(site.path(), &["--json"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json["out"].as_str().unwrap().ends_with("public"), "{json}");
    assert!(json["pages"].as_u64().unwrap() > 0, "{json}");
}

#[test]
fn a_failed_export_leaves_the_earlier_one_in_place() {
    let site = site("static");
    let settings = within(site.path(), "lint.toml");
    fs::write(settings, "[rules]\nno-authors = \"off\"\n").unwrap();
    let output = export(site.path(), &[]);
    assert_eq!(output.status.code(), Some(1), "{}", text(&output.stdout));
    let old = within(site.path(), "public/a-deleted-post/index.html");
    assert_eq!(fs::read_to_string(old).unwrap(), "old");
    assert_eq!(leftovers(site.path()), Vec::<String>::new(),);
}

#[test]
fn export_leaves_out_a_blocked_post_and_says_so() {
    let site = site("static");
    let post = within(site.path(), "content/en/posts/2026-10-20-no-author.md");
    let body = "---\ntitle: A post with no author\nslug: no-author\ndate: 2026-10-20\n---\n\nRead [why internal links matter](/blog/why-internal-links-matter/).\n";
    fs::write(post, body).unwrap();
    let output = export(site.path(), &[]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let said = text(&output.stdout);
    assert!(said.contains("This post has no author."), "{said}");
    assert!(said.contains("Left out 1 post"), "{said}");
    assert!(!within(site.path(), "public/no-author").exists());
    assert!(within(site.path(), "public/why-internal-links-matter/index.html").exists());
}

#[test]
fn a_server_site_is_not_exported() {
    let site = site("server");
    let output = export(site.path(), &[]);
    assert_eq!(output.status.code(), Some(1));
    let message = text(&output.stderr);
    let lines: Vec<&str> = message.lines().collect();
    assert_eq!(lines.len(), 3, "{message}");
    assert!(lines[0].contains("not static"), "{message}");
    assert!(lines[2].contains("mode = \"static\""), "{message}");
    let old = within(site.path(), "public/a-deleted-post/index.html");
    assert_eq!(fs::read_to_string(old).unwrap(), "old");
}

#[test]
fn export_needs_a_site_folder() {
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .arg("export")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(text(&output.stderr).contains("--site"));
}
