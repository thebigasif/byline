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

#[test]
fn new_with_draft_writes_an_undated_draft_and_says_how_to_publish_it() {
    let (temp, folder) = site();
    let args = ["--site", &folder, "new", "Why links matter", "--draft"];
    let output = khalid(&[&args[..], &["--language", "en"]].concat());
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let file = at(temp.path(), "drafts/en/why-links-matter.md");
    let written = fs::read_to_string(&file).unwrap();
    let post = parse_post(&file, &written).unwrap();
    assert_eq!(post.front.title.as_deref(), Some("Why links matter"));
    assert_eq!(post.front.date, None, "{written}");
    let posts = fs::read_dir(at(temp.path(), "content/en/posts")).map_or(0, Iterator::count);
    assert_eq!(posts, 0, "nothing goes into content/");
    let said = text(&output.stdout);
    assert!(said.contains("drafts/en/why-links-matter.md"), "{said}");
    assert!(
        said.contains("publish why-links-matter --language en"),
        "{said}"
    );

    let output = khalid(&[&args[..], &["--language", "en", "--json"]].concat());
    assert_eq!(
        output.status.code(),
        Some(1),
        "a draft is never overwritten"
    );
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    let what = json["error"]["what"].as_str().unwrap();
    assert_eq!(what, "drafts/en/why-links-matter.md already exists.");
}

#[test]
fn new_without_draft_says_the_post_goes_live_and_where_drafts_live() {
    let (_temp, folder) = site();
    let output = khalid(&["--site", &folder, "new", "Why links matter"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let said = text(&output.stdout);
    assert!(said.contains("--draft"), "{said}");
    assert!(said.contains("drafts/"), "{said}");
}

#[test]
fn new_with_language_writes_in_that_language() {
    let (temp, folder) = site();
    let output = khalid(&["--site", &folder, "new", "Über", "--language", "en"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let (name, _) = only_post(temp.path(), "en");
    assert!(name.ends_with("-uber.md"), "{name}");
}

#[test]
fn new_refuses_a_language_the_site_does_not_publish() {
    let (temp, folder) = site();
    let output = khalid(&["--site", &folder, "new", "Bonjour", "--language", "fr"]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = text(&output.stderr);
    assert_what_why_fix(&stderr);
    assert!(stderr.contains("de, en"), "{stderr}");
    assert!(!at(temp.path(), "content/fr").exists());
}

#[test]
fn new_says_which_post_already_exists() {
    let (_temp, folder) = site();
    khalid(&["--site", &folder, "new", "Twice"]);
    let output = khalid(&["--site", &folder, "new", "Twice", "--json"]);
    assert_eq!(output.status.code(), Some(1));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    let what = json["error"]["what"].as_str().unwrap();
    assert!(what.ends_with("-twice.md already exists."), "{what}");
}

/// The rules `khalid lint` finds on the one English post of the site.
#[cfg(test)]
fn rules_found(site: &Path, folder: &str) -> Vec<String> {
    let (name, _) = only_post(site, "en");
    let post = format!("content/en/posts/{name}");
    let output = khalid(&["--site", folder, "lint", &post, "--json"]);
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    json["findings"]
        .as_array()
        .unwrap_or_else(|| panic!("not a report: {json}"))
        .iter()
        .map(|finding| finding["rule"].as_str().unwrap_or_default().to_owned())
        .collect()
}

#[test]
fn a_comparison_starter_has_its_table_and_asks_for_the_answer_first() {
    let (temp, folder) = site();
    let args = ["--site", &folder, "new", "Ahrefs vs Semrush"];
    let output = khalid(&[&args[..], &["--language", "en", "--template", "comparison"]].concat());
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let (_, written) = only_post(temp.path(), "en");
    assert!(written.contains("| | Ahrefs | Semrush |"), "{written}");

    let rules = rules_found(temp.path(), &folder);
    assert!(
        rules.contains(&"comparison-answer-late".to_owned()),
        "{rules:?}"
    );
    assert!(
        !rules.contains(&"comparison-without-table".to_owned()),
        "{rules:?}"
    );
}

#[test]
fn a_review_starter_asks_for_its_rating() {
    let (temp, folder) = site();
    let args = [
        "--site",
        &folder,
        "new",
        "Ahrefs review",
        "--language",
        "en",
    ];
    let output = khalid(&[&args[..], &["--template", "review"]].concat());
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let rules = rules_found(temp.path(), &folder);
    assert!(
        rules.contains(&"review-without-rating".to_owned()),
        "{rules:?}"
    );
}

#[test]
fn new_refuses_a_template_the_title_does_not_read_as() {
    let (temp, folder) = site();
    let args = ["--site", &folder, "new", "Why links matter"];
    let output = khalid(&[&args[..], &["--template", "best-of"]].concat());
    assert_eq!(output.status.code(), Some(1));
    let stderr = text(&output.stderr);
    assert_what_why_fix(&stderr);
    assert!(stderr.contains("best-of"), "{stderr}");
    let posts = at(temp.path(), "content/de/posts");
    let written = fs::read_dir(posts).map_or(0, Iterator::count);
    assert_eq!(written, 0);
}
