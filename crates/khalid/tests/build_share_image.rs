//! `khalid build` draws a share image for every page that sets none and has
//! no site default to fall back on (KH-SEO-2, G14, task 3.7): the head names
//! it, the PNG sits in the output folder, and a second build reads the cache
//! instead of drawing again.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

/// The demo site, built once per test into a new folder, from a copy so the
/// drawing cache in `data/share/` starts empty or stays as a test left it.
#[cfg(test)]
fn built() -> TempDir {
    let (site, out) = site_copy_and_out();
    build(site.path(), out.path());
    out
}

/// A copy of the demo site in a temp folder, and a fresh output folder.
#[cfg(test)]
fn site_copy_and_out() -> (TempDir, TempDir) {
    let mut demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    demo.push("../../fixtures/demo-site");
    let site = tempfile::tempdir().unwrap();
    copy_dir(&demo, site.path());
    let out = tempfile::tempdir().unwrap();
    (site, out)
}

#[cfg(test)]
fn copy_dir(from: &Path, into: &Path) {
    fs::create_dir_all(into).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let mut target = into.to_path_buf();
        target.push(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

#[cfg(test)]
fn build(site: &Path, out: &Path) {
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.to_str().unwrap(), "build", "--out"])
        .arg(out)
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// The `<head>` of the page at `rel`.
#[cfg(test)]
fn head(out: &Path, rel: &str) -> String {
    let page = fs::read_to_string(at(out, rel)).unwrap();
    let start = page.find("<head>").unwrap();
    let end = page.find("</head>").unwrap();
    page.get(start..end).unwrap().to_owned()
}

#[cfg(test)]
fn at(base: &Path, rel: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    path.push(rel);
    path
}

/// Every `og:image` address the built home page names.
#[cfg(test)]
fn og_images(out: &Path, rel: &str) -> Vec<String> {
    let head = head(out, rel);
    head.lines()
        .filter_map(|line| line.strip_prefix(r#"<meta property="og:image" content=""#))
        .map(|rest| rest.trim_end_matches(r#"">"#).to_owned())
        .collect()
}

#[test]
fn every_page_without_an_image_gets_one_and_the_file_exists() {
    let out = built();
    for rel in [
        "index.html",
        "why-internal-links-matter/index.html",
        "about/index.html",
        "authors/asif/index.html",
        "glossary/topical-authority/index.html",
    ] {
        let images = og_images(out.path(), rel);
        assert_eq!(images.len(), 1, "{rel}: {images:?}");
        let url = &images[0];
        assert!(
            url.starts_with("https://www.example.com/blog/share/"),
            "{rel}: {url}"
        );
        let file = url.strip_prefix("https://www.example.com/blog/").unwrap();
        let png = at(out.path(), file);
        let bytes = fs::read(&png).unwrap_or_else(|error| panic!("{}: {error}", png.display()));
        assert_eq!(&bytes[..4], b"\x89PNG", "{} is a PNG", png.display());
        assert!(bytes.len() > 1000, "{} is too small", png.display());
    }
    // The 404 page stays out of search, so it draws no card.
    assert!(
        og_images(out.path(), "404.html").is_empty(),
        "the not-found page must not carry an og:image"
    );
}

#[test]
fn two_pages_with_the_same_title_and_site_share_one_image() {
    let out = built();
    let english = &og_images(out.path(), "index.html")[0];
    let german = &og_images(out.path(), "de/index.html")[0];
    assert_eq!(
        english, german,
        "both home pages show the site name, so they share one card"
    );
}

#[test]
fn a_second_build_reads_the_cache_and_draws_nothing_new() {
    let (site, out) = site_copy_and_out();
    build(site.path(), out.path());
    let share = at(site.path(), "data/share");
    let drawn: Vec<String> = sorted_files(&share);
    assert!(!drawn.is_empty(), "the first build drew cards");
    // A card is redrawn only when its PNG is missing. Remove one and touch
    // nothing else: the second build restores exactly that file.
    let victim = drawn[0].clone();
    fs::remove_file(at(&share, &victim)).unwrap();
    build(site.path(), out.path());
    assert_eq!(
        sorted_files(&share),
        drawn,
        "the second build drew only the removed card"
    );
    assert!(at(out.path(), &format!("share/{victim}")).is_file());
}

#[cfg(test)]
fn sorted_files(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(folder)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}
