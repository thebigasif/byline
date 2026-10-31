//! `khalid build` writes a snippet's blocks where a post places it, so one
//! change to the snippet shows in every post (plan 4.6, KH-EDT-1).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

/// A snippet with a heading and a callout.
const SNIPPET: &str = "---\ntitle: Affiliate disclosure\nslug: affiliate-disclosure\n---\n\n\
:::callout{kind=\"note\"}\nSome links here earn us a small fee.\n:::\n";

#[cfg(test)]
fn at(base: &Path, rel: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    path.push(rel);
    path
}

/// A post called `slug` that places `snippet` and links the other post.
#[cfg(test)]
fn post(slug: &str, snippet: &str) -> String {
    let other = if slug == "anchors" {
        "sitemaps"
    } else {
        "anchors"
    };
    format!(
        "---\ntitle: Notes on {slug}\nslug: {slug}\ndate: 2026-10-01\nauthor: asif\n---\n\n\
         Opening words.\n\n::snippet{{slug=\"{snippet}\"}}\n\nClosing words. See [[{other}]].\n"
    )
}

/// A site at `/blog/` with two posts that place the disclosure.
#[cfg(test)]
fn site() -> TempDir {
    let temp = tempfile::tempdir().unwrap();
    let settings = "name = \"Field Notes on SEO\"\naddress = \"https://www.example.com\"\n\
         mount = \"/blog/\"\nmode = \"static\"\ndefault_language = \"en\"\nlanguages = [\"en\"]\n";
    fs::write(at(temp.path(), "site.toml"), settings).unwrap();
    for folder in ["content/en/posts", "content/en/snippets"] {
        fs::create_dir_all(at(temp.path(), folder)).unwrap();
    }
    for slug in ["anchors", "sitemaps"] {
        let file = at(temp.path(), &format!("content/en/posts/{slug}.md"));
        fs::write(file, post(slug, "affiliate-disclosure")).unwrap();
    }
    let snippet = at(temp.path(), "content/en/snippets/affiliate-disclosure.md");
    fs::write(snippet, SNIPPET).unwrap();
    temp
}

#[cfg(test)]
fn build(site: &TempDir) -> Output {
    Command::new(env!("CARGO_BIN_EXE_khalid"))
        .arg("--site")
        .arg(site.path())
        .arg("build")
        .output()
        .unwrap()
}

/// The built page of post `slug`, between its opening and closing words.
#[cfg(test)]
fn middle(site: &TempDir, slug: &str) -> String {
    let output = build(site);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(0), "{stderr}{stdout}");
    let page = at(site.path(), &format!("public/{slug}/index.html"));
    let page = fs::read_to_string(page).unwrap();
    let (_, rest) = page.split_once("Opening words.").unwrap();
    rest.split_once("Closing words.").unwrap().0.to_owned()
}

#[test]
fn every_post_shows_the_snippet_and_a_change_to_it() {
    let site = site();
    for slug in ["anchors", "sitemaps"] {
        let html = middle(&site, slug);
        assert!(html.contains("callout-note"), "{slug}: {html}");
        assert!(
            html.contains("Some links here earn us a small fee."),
            "{slug}: {html}"
        );
    }
    let snippet = at(site.path(), "content/en/snippets/affiliate-disclosure.md");
    let changed = SNIPPET.replace("Some links here", "A few links here");
    fs::write(snippet, changed).unwrap();
    for slug in ["anchors", "sitemaps"] {
        let html = middle(&site, slug);
        assert!(html.contains("A few links here earn us"), "{slug}: {html}");
    }
}

#[test]
fn a_post_placing_a_missing_snippet_is_not_built() {
    let site = site();
    let file = at(site.path(), "content/en/posts/anchors.md");
    fs::write(file, post("anchors", "author-box")).unwrap();
    let output = build(&site);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_ne!(output.status.code(), Some(0), "{stderr}");
    assert!(
        stderr.contains("there is no snippet called author-box in en"),
        "{stderr}"
    );
    assert!(
        stderr.contains("content/en/snippets/author-box.md"),
        "{stderr}"
    );
}
