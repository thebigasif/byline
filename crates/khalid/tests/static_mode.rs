//! A static site (`mode = "static"`, KH-OPS-1): `khalid build` writes the
//! demo site and says which features are off and why.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;

#[cfg(test)]
fn at(base: &Path, rel: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    path.push(rel);
    path
}

/// A copy of the demo site in `mode`.
#[cfg(test)]
fn demo(mode: &str) -> TempDir {
    let demo = at(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        "../../fixtures/demo-site",
    );
    let temp = tempfile::tempdir().unwrap();
    copy_dir(&demo, temp.path());
    let settings = at(temp.path(), "site.toml");
    let text = fs::read_to_string(&settings).unwrap();
    let moded = text.replace("mode = \"server\"", &format!("mode = \"{mode}\""));
    fs::write(&settings, moded).unwrap();
    temp
}

#[cfg(test)]
fn copy_dir(from: &Path, into: &Path) {
    fs::create_dir_all(into).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        // A demo built in place keeps image sizes in data/; a copy starts clean.
        if name == "data" {
            continue;
        }
        let target = at(into, name.to_str().unwrap());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// Runs `khalid build` on `site` into `out`, with `--json` first if asked.
#[cfg(test)]
fn build(site: &Path, out: &Path, json: &[&str]) -> Output {
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(json)
        .arg("--site")
        .arg(site)
        .arg("build")
        .arg("--out")
        .arg(out)
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

#[test]
fn a_static_build_says_which_features_are_off_and_why() {
    let site = demo("static");
    let out = tempfile::tempdir().unwrap();
    let said = String::from_utf8_lossy(&build(site.path(), out.path(), &[]).stdout).into_owned();
    assert!(said.starts_with("Built "), "{said}");
    assert!(
        said.contains("The site is static, so 6 features that need a server are off."),
        "{said}"
    );
    for what in [
        "Comments are off.",
        "Newsletter sign-up is off.",
        "Forms are off.",
        "Search is off.",
        "The admin is not part of the built site.",
        "Scheduled publish is off.",
    ] {
        assert!(said.contains(what), "{what} in {said}");
    }
    assert!(
        said.contains(
            "Scheduled publish is off.\n  A static site changes only when you build it, so a post set for later waits for the next build.\n  Run `khalid build` on a schedule, or set mode = \"server\" in site.toml and run `khalid serve`.\n"
        ),
        "{said}"
    );
}

#[test]
fn a_static_build_lists_the_features_off_in_json() {
    let site = demo("static");
    let out = tempfile::tempdir().unwrap();
    let output = build(site.path(), out.path(), &["--json"]);
    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    let off = body["off"].as_array().unwrap();
    let features: Vec<&str> = off
        .iter()
        .map(|lock| lock["feature"].as_str().unwrap())
        .collect();
    assert_eq!(
        features,
        [
            "comments",
            "newsletter",
            "forms",
            "search",
            "admin",
            "scheduled_publish"
        ]
    );
    for lock in off {
        for part in ["what", "why", "fix"] {
            assert!(lock[part].as_str().is_some_and(|s| !s.is_empty()), "{lock}");
        }
    }
}

#[test]
fn a_server_build_lists_nothing_off() {
    let site = demo("server");
    let out = tempfile::tempdir().unwrap();
    let output = build(site.path(), out.path(), &["--json"]);
    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(body.get("off").is_none(), "{body}");
    let said = String::from_utf8_lossy(&build(site.path(), out.path(), &[]).stdout).into_owned();
    assert!(!said.contains("features that need a server"), "{said}");
}
