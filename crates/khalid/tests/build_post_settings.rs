//! `khalid build` with a post's own settings: the theme template it asks
//! for (KH-EDT-3).

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
