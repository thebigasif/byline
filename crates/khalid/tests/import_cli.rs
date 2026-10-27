//! `khalid import` (KH-API-7): a dry run writes nothing, an import writes
//! the posts with their slugs and a redirect map `khalid redirects import`
//! stores, and a second import refuses to overwrite.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

/// A site with the demo's settings and no content yet.
#[cfg(test)]
fn site() -> TempDir {
    let settings = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/demo-site/site.toml"
    );
    let temp = tempfile::tempdir().unwrap();
    fs::copy(settings, file(temp.path(), "site.toml")).unwrap();
    fs::create_dir(file(temp.path(), "content")).unwrap();
    temp
}

/// A fixture of the importer crate, by its path inside `fixtures/`.
#[cfg(test)]
fn fixture(name: &str) -> String {
    format!(
        "{}/../khalid-import/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    )
}

/// Runs `khalid --site <site>` with `args`, and returns the exit code and
/// what it printed.
#[cfg(test)]
fn khalid(site: &Path, args: &[&str]) -> (i32, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.to_str().unwrap()])
        .args(args)
        .output()
        .unwrap();
    let mut printed = String::from_utf8(output.stdout).unwrap();
    printed.push_str(&String::from_utf8(output.stderr).unwrap());
    (output.status.code().unwrap(), printed)
}

/// A file inside `site`.
#[cfg(test)]
fn file(site: &Path, path: &str) -> PathBuf {
    let mut file = site.to_path_buf();
    file.push(path);
    file
}

const POST: &str = "content/en/posts/2024-03-12-keep-a-starter-alive.md";

#[test]
fn a_dry_run_reports_and_writes_nothing() {
    let site = site();
    let export = fixture("wordpress/basic.xml");
    let (code, printed) = khalid(site.path(), &["import", "wordpress", &export, "--dry-run"]);
    assert_eq!(code, 0, "{printed}");
    assert!(
        printed.starts_with("This is a dry run: nothing was written."),
        "{printed}"
    );
    assert!(printed.contains("Slugs kept:"), "{printed}");
    assert!(!file(site.path(), "content/en").exists());
    assert!(!file(site.path(), "data").exists());
}

#[test]
fn a_wordpress_import_writes_posts_and_a_map_redirects_import_stores() {
    let site = site();
    let export = fixture("wordpress/basic.xml");
    let rules = fixture("wordpress/redirection.json");
    let args = ["import", "wordpress", &export, "--redirection", &rules];
    let (code, printed) = khalid(site.path(), &args);
    assert_eq!(code, 0, "{printed}");
    assert!(!printed.contains("dry run"), "{printed}");
    let post = fs::read_to_string(file(site.path(), POST)).unwrap();
    assert!(post.contains("slug: \"keep-a-starter-alive\""), "{post}");
    let map = fs::read_to_string(file(site.path(), "import/wordpress-redirects.csv")).unwrap();
    let rows = map.lines().count() - 1;
    assert!(rows > 0, "{map}");
    let told = "Run `khalid redirects import import/wordpress-redirects.csv`";
    assert!(printed.contains(told), "{printed}");
    let map_file = file(site.path(), "import/wordpress-redirects.csv");
    let (code, done) = khalid(
        site.path(),
        &["redirects", "import", map_file.to_str().unwrap()],
    );
    assert_eq!(code, 0, "{done}");
    assert!(done.contains(&format!("Stored {rows} redirects")), "{done}");
    let (code, stored) = khalid(site.path(), &["redirects", "export"]);
    assert_eq!(code, 0, "{stored}");
    assert_eq!(
        stored.lines().count(),
        map.lines().count(),
        "{stored}\n{map}"
    );

    let (code, again) = khalid(site.path(), &["import", "wordpress", &export]);
    assert_eq!(code, 1, "{again}");
    assert!(again.contains(POST), "{again}");
}

#[test]
fn a_ghost_import_answers_in_json() {
    let site = site();
    let export = fixture("ghost/ghost-5.json");
    let args = [
        "--json",
        "import",
        "ghost",
        &export,
        "--old-site",
        "https://old.example.com",
    ];
    let (code, printed) = khalid(site.path(), &args);
    assert_eq!(code, 0, "{printed}");
    let body: Value = serde_json::from_str(&printed).unwrap();
    assert_eq!(body["dry_run"], false);
    let written: Vec<&str> = body["written"]
        .as_array()
        .unwrap()
        .iter()
        .map(|path| path.as_str().unwrap())
        .collect();
    assert!(written.contains(&"import/ghost-redirects.csv"), "{body}");
    assert!(written.iter().all(|path| file(site.path(), path).is_file()));
}

#[test]
fn an_export_that_is_not_one_imports_nothing() {
    let site = site();
    let export = fixture("ghost/ghost-5.json");
    let (code, printed) = khalid(site.path(), &["import", "wordpress", &export]);
    assert_eq!(code, 1, "{printed}");
    assert!(printed.contains("Nothing was imported"), "{printed}");
    assert!(!file(site.path(), "content/en").exists());
}
