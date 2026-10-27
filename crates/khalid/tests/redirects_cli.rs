//! `khalid redirects import`, `export` and `explain` (KH-SEO-8): a list
//! goes in whole or not at all, comes back out as the same CSV, and one
//! address is explained in plain words.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

/// A site with the demo's settings and no content: the redirect commands
/// read only the settings and the database.
#[cfg(test)]
fn site() -> TempDir {
    let mut settings = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    settings.push("../../fixtures/demo-site/site.toml");
    let temp = tempfile::tempdir().unwrap();
    let mut into = temp.path().to_path_buf();
    into.push("site.toml");
    fs::copy(settings, into).unwrap();
    temp
}

/// Writes `csv` into `site` as `name`, and returns its path.
#[cfg(test)]
fn list(site: &Path, name: &str, csv: &str) -> PathBuf {
    let mut path = site.to_path_buf();
    path.push(name);
    fs::write(&path, csv).unwrap();
    path
}

/// Runs `khalid --site <site> redirects` with `args`, and returns the exit
/// code and what it printed.
#[cfg(test)]
fn redirects(site: &Path, args: &[&str]) -> (i32, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.to_str().unwrap(), "redirects"])
        .args(args)
        .output()
        .unwrap();
    let mut printed = String::from_utf8(output.stdout).unwrap();
    printed.push_str(&String::from_utf8(output.stderr).unwrap());
    (output.status.code().unwrap(), printed)
}

const IMPORTED: &str = "from,to,status,match\n\
    /blog/old-links/,/blog/why-internal-links-matter/,301,exact\n\
    /blog/2019/(.+),/blog/$1,302,regex\n\
    /old-site/about/,https://serpdad.com/about/,301,exact\n";

#[test]
fn a_list_goes_in_comes_out_the_same_and_explains_an_address() {
    let site = site();
    let file = list(site.path(), "wordpress-redirects.csv", IMPORTED);
    let (code, printed) = redirects(site.path(), &["import", file.to_str().unwrap()]);
    assert_eq!(code, 0, "{printed}");
    assert!(printed.contains("Stored 3 redirects"), "{printed}");
    assert!(
        printed.contains("1 of them start outside /blog/."),
        "{printed}"
    );

    let (code, printed) = redirects(site.path(), &["export"]);
    assert_eq!(code, 0, "{printed}");
    let exact_rules_then_patterns = "from,to,status,match\n\
        /blog/old-links/,/blog/why-internal-links-matter/,301,exact\n\
        /old-site/about/,https://serpdad.com/about/,301,exact\n\
        /blog/2019/(.+),/blog/$1,302,regex\n";
    assert_eq!(printed, exact_rules_then_patterns);

    let (code, printed) = redirects(site.path(), &["explain", "/blog/2019/old-links/"]);
    assert_eq!(code, 0, "{printed}");
    assert!(
        printed.contains("matches rule 2, the pattern /blog/2019/(.+)."),
        "{printed}"
    );
    assert!(
        printed.ends_with(
            "A visitor gets one step: moves for now (302) to /blog/why-internal-links-matter/.\n"
        ),
        "{printed}"
    );

    let (code, printed) = redirects(site.path(), &["explain", "/blog/2019/old-links/", "--json"]);
    assert_eq!(code, 0, "{printed}");
    let json: Value = serde_json::from_str(&printed).unwrap();
    assert_eq!(
        (json["status"].as_u64(), json["to"].as_str()),
        (Some(302), Some("/blog/why-internal-links-matter/"))
    );
}

#[test]
fn a_bad_row_a_loop_or_a_bad_address_stores_nothing_and_says_why() {
    let site = site();
    let bad = list(site.path(), "bad.csv", "from,to,status\n/blog/a/,,301\n");
    let (code, printed) = redirects(site.path(), &["import", bad.to_str().unwrap()]);
    assert_eq!(code, 1, "{printed}");
    assert!(printed.contains("line 2"), "{printed}");

    let looped = list(
        site.path(),
        "loop.csv",
        "from,to,status\n/blog/a/,/blog/b/,301\n/blog/b/,/blog/a/,301\n",
    );
    let (code, printed) = redirects(site.path(), &["import", looped.to_str().unwrap()]);
    assert_eq!(code, 1, "{printed}");
    assert!(printed.contains("loop"), "{printed}");
    let (_, printed) = redirects(site.path(), &["export"]);
    assert_eq!(printed, "from,to,status,match\n");

    let (code, printed) = redirects(site.path(), &["explain", "blog/no-slash/"]);
    assert_eq!(code, 2, "{printed}");
    let (code, printed) = redirects(site.path(), &["import", "missing.csv"]);
    assert_eq!(code, 2, "{printed}");
}
