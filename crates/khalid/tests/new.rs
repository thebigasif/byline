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

#[test]
fn new_with_author_writes_the_byline_and_makes_the_profile_once() {
    let (temp, folder) = site();
    let args = ["--site", &folder, "new", "Why links matter"];
    let output = khalid(&[&args[..], &["--author", "Sam Lee"]].concat());
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let said = text(&output.stdout);
    assert!(said.contains("content/de/authors/sam-lee.md"), "{said}");
    let (name, written) = only_post(temp.path(), "de");
    let post = parse_post(Path::new(&name), &written).unwrap();
    assert_eq!(post.front.author.unwrap().as_str(), "sam-lee");
    let profile = at(temp.path(), "content/de/authors/sam-lee.md");
    let made = fs::read_to_string(&profile).unwrap();
    assert!(made.contains("name: \"Sam Lee\"\n"), "{made}");
    assert!(made.contains("slug: \"sam-lee\"\n"), "{made}");

    // The writer's own words in the profile are never overwritten.
    fs::write(&profile, format!("{made}Sam writes here.\n")).unwrap();
    let args = [
        "--site",
        &folder,
        "new",
        "Second post",
        "--author",
        "Sam Lee",
    ];
    assert_eq!(khalid(&args).status.code(), Some(0));
    let kept = fs::read_to_string(&profile).unwrap();
    assert!(kept.ends_with("Sam writes here.\n"), "{kept}");
}

#[test]
fn new_without_author_takes_the_only_profile_or_says_how_to_add_one() {
    let (temp, folder) = site();
    let output = khalid(&["--site", &folder, "new", "First post", "--json"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["author"], Value::Null, "{json}");
    let said = text(&khalid(&["--site", &folder, "new", "Second post"]).stdout);
    assert!(said.contains("--author \"Your name\""), "{said}");

    let profile = at(temp.path(), "content/de/authors/asif.md");
    fs::create_dir_all(profile.parent().unwrap()).unwrap();
    fs::write(&profile, "---\nname: Asif\nslug: asif\n---\n").unwrap();
    let output = khalid(&["--site", &folder, "new", "Third post", "--json"]);
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["author"], "asif", "{json}");
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
