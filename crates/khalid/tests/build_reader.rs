//! `khalid build` fills in the reader features of plan section 4.12 from the
//! content and `site.toml`: bylines with roles, reading time, the author box
//! and the other authors, each one off when `[reader]` says so.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(test)]
fn at(base: &Path, rel: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    path.push(rel);
    path
}

/// Copies the folder `from` into `into`, file by file.
#[cfg(test)]
fn copy_dir(from: &Path, into: &Path) {
    fs::create_dir_all(into).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = at(into, &entry.file_name().to_string_lossy());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// A copy of the demo site where Asif reviewed the sitemap post, with
/// `reader` appended to `site.toml`, built into `out`.
#[cfg(test)]
fn build(reader: &str) -> (tempfile::TempDir, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let site = at(temp.path(), "site");
    let demo = at(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        "../../fixtures/demo-site",
    );
    copy_dir(&demo, &site);
    let post = at(&site, "content/en/posts/2026-09-28-what-a-sitemap-does.md");
    let text = fs::read_to_string(&post).unwrap();
    fs::write(
        &post,
        text.replacen(
            "\nauthor: lena-vogt\n",
            "\nauthor: lena-vogt\nreviewed_by: asif\n",
            1,
        ),
    )
    .unwrap();
    let settings = at(&site, "site.toml");
    let mut toml = fs::read_to_string(&settings).unwrap();
    toml.push_str(reader);
    fs::write(&settings, toml).unwrap();
    let out = at(temp.path(), "public");
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args([
            "--site",
            site.to_str().unwrap(),
            "build",
            "--out",
            out.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    (temp, out)
}

#[cfg(test)]
fn page(out: &Path, rel: &str) -> String {
    fs::read_to_string(at(out, rel)).unwrap_or_else(|error| panic!("{rel}: {error}"))
}

#[test]
fn a_post_shows_its_author_with_role_and_bio_and_its_reading_time() {
    let (_temp, out) = build("");
    let post = page(&out, "what-a-sitemap-does/index.html");
    assert!(
        post.contains("<span class=\"person-role\">Technical SEO editor</span>"),
        "{post}"
    );
    assert!(post.contains("min read</time>"), "{post}");
    assert!(post.contains("<section class=\"author-box\">"), "{post}");
    assert!(
        post.contains("She writes in English and German."),
        "the bio:\n{post}"
    );
    let home = page(&out, "index.html");
    assert!(
        home.contains("<span class=\"card-author\">Lena Vogt</span>"),
        "cards name their author:\n{home}"
    );
}

#[test]
fn an_author_page_ends_with_the_other_authors() {
    let (_temp, out) = build("");
    let lena = page(&out, "authors/lena-vogt/index.html");
    let others = lena
        .split("<h2 id=\"other-authors\">")
        .nth(1)
        .expect("other authors");
    assert!(others.contains("/blog/authors/asif/"), "{others}");
    assert!(!others.contains("/blog/authors/lena-vogt/"), "{others}");
}

#[test]
fn features_switched_off_in_site_toml_are_gone() {
    let (_temp, out) = build("\n[reader]\nauthor_box = false\nreading_time = false\n");
    let post = page(&out, "what-a-sitemap-does/index.html");
    for gone in ["author-box", "min read"] {
        assert!(!post.contains(gone), "{gone} is still there:\n{post}");
    }
    assert!(post.contains("Technical SEO editor"), "the byline stays on");
}
