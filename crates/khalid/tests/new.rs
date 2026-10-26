//! `khalid new "Title"`: a post file with correct front matter and a
//! transliterated slug (RULES.md BL-CLI-1 to BL-CLI-3, BL-RUN-9).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use khalid_content::parse_post;
use serde_json::Value;

#[cfg(test)]
fn khalid(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(args)
        .output()
        .unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn at(base: &Path, rel: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    path.push(rel);
    path
}

/// A new German and English site in a temporary folder, German first.
#[cfg(test)]
fn site() -> (tempfile::TempDir, String) {
    let temp = tempfile::tempdir().unwrap();
    let folder = temp.path().to_str().unwrap().to_owned();
    let made = khalid(&[
        "init",
        &folder,
        "--address",
        "https://www.example.com",
        "--languages",
        "de,en",
    ]);
    assert_eq!(made.status.code(), Some(0), "{}", text(&made.stderr));
    (temp, folder)
}

/// The one file in a language's posts folder, by name and text.
#[cfg(test)]
fn only_post(site: &Path, language: &str) -> (String, String) {
    let folder = at(site, &format!("content/{language}/posts"));
    let mut names: Vec<String> = fs::read_dir(&folder)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(names.len(), 1, "{names:?}");
    let name = names.remove(0);
    let text = fs::read_to_string(at(&folder, &name)).unwrap();
    (name, text)
}

/// Checks a message the way BL-CLI-3 asks: three short sentences.
fn assert_what_why_fix(stderr: &str) {
    let lines: Vec<&str> = stderr.lines().collect();
    assert_eq!(lines.len(), 3, "want what, why, fix:\n{stderr}");
    for line in lines {
        assert!(line.ends_with('.'), "{line:?}");
        assert!(line.split_whitespace().count() < 25, "{line:?}");
    }
}

#[test]
fn new_writes_a_post_with_a_transliterated_slug() {
    let (temp, folder) = site();
    let output = khalid(&["--site", &folder, "new", "Über Größe: der \"Test\""]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));

    let (name, written) = only_post(temp.path(), "de");
    let post = parse_post(Path::new(&name), &written).unwrap();
    assert_eq!(
        post.front.title.as_deref(),
        Some("Über Größe: der \"Test\"")
    );
    let slug = post.front.slug.unwrap();
    assert_eq!(slug.as_str(), "ueber-groesse-der-test");
    let date = post.front.date.unwrap();
    assert_eq!(name, format!("{date}-{slug}.md"));
    assert!(
        text(&output.stdout).contains(&name),
        "{}",
        text(&output.stdout)
    );

    let checked = khalid(&["--site", &folder]);
    assert_eq!(checked.status.code(), Some(0), "{}", text(&checked.stderr));
}

#[test]
fn new_with_json_prints_what_it_wrote() {
    let (temp, folder) = site();
    let output = khalid(&["--site", &folder, "new", "Why links matter", "--json"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    let (name, _) = only_post(temp.path(), "de");
    assert_eq!(json["path"], format!("content/de/posts/{name}"));
    assert_eq!(json["title"], "Why links matter");
    assert_eq!(json["slug"], "why-links-matter");
    assert_eq!(json["language"], "de");
    assert!(name.starts_with(json["date"].as_str().unwrap()));
}

#[test]
fn new_never_overwrites_a_post() {
    let (temp, folder) = site();
    assert_eq!(
        khalid(&["--site", &folder, "new", "Twice"]).status.code(),
        Some(0)
    );
    let (name, first) = only_post(temp.path(), "de");
    fs::write(
        at(temp.path(), &format!("content/de/posts/{name}")),
        format!("{first}Mine.\n"),
    )
    .unwrap();

    let output = khalid(&["--site", &folder, "new", "Twice"]);
    assert_eq!(output.status.code(), Some(1));
    assert_what_why_fix(&text(&output.stderr));
    let (_, kept) = only_post(temp.path(), "de");
    assert!(kept.ends_with("Mine.\n"), "{kept}");
}

#[test]
fn new_refuses_a_title_with_nothing_to_make_a_slug_from() {
    let (_temp, folder) = site();
    let output = khalid(&["--site", &folder, "new", "!!!", "--json"]);
    assert_eq!(output.status.code(), Some(1));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json["error"]["what"].as_str().unwrap().contains("!!!"));
}

#[test]
fn new_needs_a_site_folder() {
    let output = khalid(&["new", "Why links matter"]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = text(&output.stderr);
    assert_what_why_fix(&stderr);
    assert!(stderr.contains("--site"), "{stderr}");
}
