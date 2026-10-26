//! `khalid build` applies the index settings of `[seo.index]` in `site.toml`
//! to the robots tag of each archive page (KH-SEO-9, audit brief B-SEO-9).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

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

/// The demo site with `settings` added to the end of its `site.toml`, built
/// into a new folder. Returns the site and the output folder.
#[cfg(test)]
fn built_with(settings: &str) -> (TempDir, TempDir) {
    let site = tempfile::tempdir().unwrap();
    let demo = at(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        "../../fixtures/demo-site",
    );
    copy_dir(&demo, site.path());
    let toml = at(site.path(), "site.toml");
    let text = fs::read_to_string(&toml).unwrap();
    fs::write(&toml, format!("{text}\n{settings}")).unwrap();
    let out = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.path().to_str().unwrap(), "build", "--out"])
        .arg(out.path())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    (site, out)
}

/// The robots and canonical tags of the built page at `rel`.
#[cfg(test)]
fn robots_and_canonical(out: &TempDir, rel: &str) -> String {
    let page = fs::read_to_string(at(out.path(), rel)).unwrap();
    page.lines()
        .filter(|line| {
            line.starts_with("<meta name=\"robots\"") || line.starts_with("<link rel=\"canonical\"")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn author_archives_set_to_noindex_are_kept_out_of_search() {
    let (_site, out) = built_with("[seo.index]\nauthors = \"noindex\"\n");
    insta::assert_snapshot!(robots_and_canonical(&out, "authors/asif/index.html"));
}

#[test]
fn an_author_with_fewer_posts_than_the_thin_limit_is_kept_out_of_search() {
    let (_site, out) = built_with("[seo.index]\nthin_archives_below = 50\n");
    let tags = robots_and_canonical(&out, "authors/asif/index.html");
    assert!(
        tags.contains(r#"<meta name="robots" content="noindex, follow">"#),
        "{tags}"
    );
}

#[test]
fn the_archive_settings_leave_posts_indexed() {
    let (_site, out) = built_with("[seo.index]\nauthors = \"noindex\"\n");
    let tags = robots_and_canonical(&out, "why-internal-links-matter/index.html");
    assert!(
        tags.contains(r#"<meta name="robots" content="index, follow"#),
        "{tags}"
    );
}
