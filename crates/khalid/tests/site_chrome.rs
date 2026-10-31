//! What every page shows around its content, from `site.toml`: the menus
//! in the header and footer, each link in its language. Plain HTML, so it
//! reads the same with scripts off.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

/// Two menus, with one footer link for German pages only.
const MENUS: &str = r#"
[[nav.primary]]
label = "Guides"
href = "/blog/guides/"

[[nav.primary]]
label = "Shop"
href = "https://shop.example.com/"

[[nav.footer]]
label = "Privacy"
href = "/blog/privacy/"

[[nav.footer]]
label = "Impressum"
href = "/blog/de/impressum/"
language = "de"
"#;

#[cfg(test)]
fn at(base: &Path, rel: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    path.push(rel);
    path
}

#[cfg(test)]
fn copy_dir(from: &Path, into: &Path) {
    fs::create_dir_all(into).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = at(into, &entry.file_name().to_string_lossy());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// Builds a copy of the demo site with `extra` appended to `site.toml`.
#[cfg(test)]
fn build(extra: &str) -> (TempDir, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let demo = at(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        "../../fixtures/demo-site",
    );
    let site = at(temp.path(), "site");
    copy_dir(&demo, &site);
    let settings = at(&site, "site.toml");
    let mut toml = fs::read_to_string(&settings).unwrap();
    toml.push_str(extra);
    fs::write(&settings, toml).unwrap();
    let out = at(temp.path(), "public");
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .arg("--site")
        .arg(&site)
        .arg("build")
        .arg("--out")
        .arg(&out)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    (temp, out)
}

#[cfg(test)]
fn page(out: &Path, rel: &str) -> String {
    fs::read_to_string(at(out, rel)).unwrap_or_else(|error| panic!("{rel}: {error}"))
}

/// The part of `html` from `start` to the first `end` after it.
#[cfg(test)]
fn part<'h>(html: &'h str, start: &str, end: &str) -> &'h str {
    let from = html.find(start).unwrap_or_else(|| panic!("no {start}"));
    let (_, rest) = html.split_at(from);
    let to = rest.find(end).unwrap_or_else(|| panic!("no {end}")) + end.len();
    rest.split_at(to).0
}

#[test]
fn every_page_shows_the_menus_of_site_toml() {
    let (_temp, out) = build(MENUS);
    let home = page(&out, "index.html");
    let header = part(&home, "<header", "</header>");
    let footer = part(&home, "<footer", "</footer>");
    insta::assert_snapshot!(format!("{header}\n{footer}"));
    let post = page(&out, "what-a-sitemap-does/index.html");
    assert!(post.contains(header), "the post has the same header");
}

#[test]
fn a_link_for_one_language_shows_only_on_its_pages() {
    let (_temp, out) = build(MENUS);
    let english = page(&out, "index.html");
    let german = page(&out, "de/index.html");
    assert!(!english.contains("Impressum"), "{english}");
    let footer = part(&german, "<footer", "</footer>");
    assert!(
        footer.contains("<a href=\"/blog/privacy/\">Privacy</a>"),
        "{footer}"
    );
    assert!(
        footer.contains("<a href=\"/blog/de/impressum/\">Impressum</a>"),
        "{footer}"
    );
}

#[test]
fn a_menu_link_to_the_page_shown_is_marked_current() {
    let (_temp, out) =
        build("\n[[nav.primary]]\nlabel = \"Sitemaps\"\nhref = \"/blog/what-a-sitemap-does/\"\n");
    let post = page(&out, "what-a-sitemap-does/index.html");
    assert!(
        post.contains("<a href=\"/blog/what-a-sitemap-does/\" aria-current=\"page\">Sitemaps</a>"),
        "{post}"
    );
}

#[test]
fn a_site_without_menus_shows_no_menu() {
    let (_temp, out) = build("");
    let home = page(&out, "index.html");
    let header = part(&home, "<header", "</header>");
    assert!(!header.contains("<nav"), "{header}");
    assert!(
        !home.contains("class=\"announcement\""),
        "no bar by default"
    );
}

#[test]
fn the_announcement_sits_above_the_header_of_every_page() {
    let (_temp, out) = build(
        "\n[announcement]\ntext = \"Our technical SEO course opens on 3 November.\"\nhref = \"/blog/course/\"\n",
    );
    let home = page(&out, "index.html");
    let bar = part(&home, "<a class=\"skip-link\"", "<header");
    insta::assert_snapshot!(bar);
    for other in [
        "what-a-sitemap-does/index.html",
        "de/index.html",
        "404.html",
    ] {
        let html = page(&out, other);
        assert!(html.contains("course opens on 3 November."), "{other}");
    }
}

#[test]
fn an_announcement_for_one_language_is_text_alone_on_its_pages() {
    let (_temp, out) = build("\n[announcement]\ntext = \"Neu: der Kurs.\"\nlanguage = \"de\"\n");
    let german = page(&out, "de/index.html");
    assert!(
        german.contains(
            "<aside class=\"announcement\" aria-label=\"Mitteilung\"><p>Neu: der Kurs.</p></aside>"
        ),
        "{german}"
    );
    assert!(!page(&out, "index.html").contains("announcement"));
}
