//! `khalid build` with a post's own settings: the theme template it asks
//! for and its head code (KH-EDT-3).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[cfg(test)]
fn khalid(site: &Path) -> Output {
    let site = site.to_str().unwrap();
    Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site, "build"])
        .output()
        .unwrap()
}

#[cfg(test)]
fn at(base: &Path, rel: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    path.push(rel);
    path
}

/// A site with the demo settings, one post whose front matter ends with
/// `extra`, and the theme templates `templates`.
#[cfg(test)]
fn site(extra: &str, templates: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let mut demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    demo.push("../../fixtures/demo-site/site.toml");
    fs::copy(demo, at(dir.path(), "site.toml")).unwrap();
    let posts = at(dir.path(), "content/en/posts");
    fs::create_dir_all(&posts).unwrap();
    let post = format!(
        "---\ntitle: Why internal links matter\ndate: 2026-09-14\nauthor: asif\n\
         description: Internal links help readers find the rest of your site.\n{extra}---\n\n\
         An internal link points to [the front page](/blog/).\n"
    );
    fs::write(at(&posts, "2026-09-14-why-internal-links-matter.md"), post).unwrap();
    let theme = at(dir.path(), "theme/templates");
    fs::create_dir_all(&theme).unwrap();
    for (name, text) in templates {
        fs::write(at(&theme, name), text).unwrap();
    }
    dir
}

#[cfg(test)]
fn post_page(site: &Path) -> String {
    fs::read_to_string(at(site, "public/why-internal-links-matter/index.html")).unwrap()
}

const WIDE: &str = r#"{% extends "post.html" %}{% block main %}<div class="wide">{{ page.title }}</div>{% endblock %}"#;

#[test]
fn a_post_is_drawn_with_the_template_it_asks_for() {
    let site = site("template: wide\n", &[("post-wide.html", WIDE)]);
    let output = khalid(site.path());
    let said = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(0), "{said}");
    let page = post_page(site.path());
    assert!(
        page.contains("<div class=\"wide\">Why internal links matter</div>"),
        "{page}"
    );
}

#[test]
fn a_post_without_a_template_keeps_the_default() {
    let site = site("", &[("post-wide.html", WIDE)]);
    assert_eq!(khalid(site.path()).status.code(), Some(0));
    assert!(!post_page(site.path()).contains("class=\"wide\""));
}

#[test]
fn head_code_goes_after_the_seo_head_of_its_post_alone() {
    let code = "head_code: \"<meta name=verify content=abc>\\n<script>go()</script>\"\n";
    let site = site(code, &[]);
    let output = khalid(site.path());
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let page = post_page(site.path());
    let head = page.split("</head>").next().unwrap();
    let written = "<meta name=\"verify\" content=\"abc\">\n<script>go()</script>";
    assert!(
        head.contains(written),
        "a static page gets no nonce: {head}"
    );
    assert!(
        head.find("application/ld+json") < head.find(written),
        "{head}"
    );
    assert!(!page.contains("khalid-nonce-slot"), "{page}");
    let home = fs::read_to_string(at(site.path(), "public/index.html")).unwrap();
    assert!(!home.contains("verify"), "{home}");
}

/// The body of the test post, which a locked page must never show.
const BODY: &str = "An internal link points to";

/// Every file `khalid build` wrote, as text, with its path.
#[cfg(test)]
fn every_file(dir: &Path, into: &mut Vec<(PathBuf, String)>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            every_file(&path, into);
        } else if let Ok(text) = fs::read_to_string(&path) {
            into.push((path, text));
        }
    }
}

#[cfg(test)]
fn nothing_shows_the_body(site: &Path) {
    let mut files = Vec::new();
    every_file(&at(site, "public"), &mut files);
    assert!(files.len() > 5, "{files:?}");
    for (path, text) in files {
        assert!(!text.contains(BODY), "{} shows the body", path.display());
    }
}

#[test]
fn a_members_post_shows_its_description_and_a_note_and_no_body_anywhere() {
    let site = site("access: members\n", &[]);
    assert_eq!(khalid(site.path()).status.code(), Some(0));
    let page = post_page(site.path());
    assert!(
        page.contains("Internal links help readers find the rest of your site."),
        "{page}"
    );
    assert!(page.contains("This post is for members."), "{page}");
    assert!(!page.contains("<form class=\"unlock\""), "{page}");
    nothing_shows_the_body(site.path());
}

#[test]
fn a_password_post_shows_a_form_that_works_without_scripts() {
    let hash = "$argon2id$v=19$m=19456,t=2,p=1$c2FsdHNhbHQ$aGFzaGhhc2g";
    let site = site(&format!("password_hash: \"{hash}\"\n"), &[]);
    assert_eq!(khalid(site.path()).status.code(), Some(0));
    let page = post_page(site.path());
    assert!(
        page.contains("<form class=\"unlock\" method=\"post\" action=\"/blog/_unlock/\">"),
        "{page}"
    );
    assert!(page.contains("type=\"password\""), "{page}");
    assert!(
        page.contains("<meta name=\"robots\" content=\"noindex"),
        "{page}"
    );
    assert!(!page.contains("argon2"), "{page}");
    nothing_shows_the_body(site.path());
}

#[test]
fn a_static_site_shows_a_password_post_locked_with_no_form_to_send() {
    let hash = "$argon2id$v=19$m=19456,t=2,p=1$c2FsdHNhbHQ$aGFzaGhhc2g";
    let site = site(&format!("password_hash: \"{hash}\"\n"), &[]);
    let settings = at(site.path(), "site.toml");
    let text = fs::read_to_string(&settings).unwrap();
    fs::write(
        &settings,
        text.replace("mode = \"server\"", "mode = \"static\""),
    )
    .unwrap();
    assert_eq!(khalid(site.path()).status.code(), Some(0));
    let page = post_page(site.path());
    assert!(
        !page.contains("_unlock/"),
        "no form posts to a missing server: {page}"
    );
    assert!(
        page.contains("This form cannot be sent from this site."),
        "{page}"
    );
    assert!(!at(site.path(), "public/_unlocked").exists());
    nothing_shows_the_body(site.path());
}

#[test]
fn a_template_the_theme_lacks_stops_the_build_and_names_the_file() {
    let site = site("template: narrow\n", &[]);
    let output = khalid(site.path());
    assert_eq!(output.status.code(), Some(1));
    let said = String::from_utf8_lossy(&output.stderr);
    assert!(
        said.contains("theme/templates/post-narrow.html does not exist"),
        "{said}"
    );
    assert!(
        said.contains("2026-09-14-why-internal-links-matter.md"),
        "{said}"
    );
    assert_eq!(said.lines().count(), 3, "what, why, fix:\n{said}");
}
