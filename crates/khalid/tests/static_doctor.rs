//! `khalid doctor` on a static site (KH-OPS-1, KH-OPS-2): it lists the same
//! features `khalid build` says are off, with what, why and fix, and a
//! static site is not a problem to fix.

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

/// A copy of the demo site's files in `mode`, without its `data/` folder.
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

#[cfg(test)]
fn khalid(args: &[&str], site: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(args)
        .arg("--site")
        .arg(site)
        .args(["doctor", "--offline"])
        .output()
        .unwrap()
}

#[test]
fn doctor_on_a_static_site_lists_the_features_off_and_finds_no_problem() {
    let site = demo("static");
    let output = khalid(&[], site.path());
    let said = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(0), "{said}");
    let heading = "The site is static, so 6 features that need a server are off.";
    assert!(said.contains(heading), "{said}");
    let off = said.find(heading).unwrap();
    let address = said.find("Public address: skipped.").unwrap();
    assert!(address < off, "{said}");
    assert!(
        said.contains(
            "Comments are off.\n  A static host only sends files, so nothing can keep a reader's comment, and each post shows a short note instead.\n  To turn this feature on, set mode = \"server\" in site.toml and run `khalid serve`.\n"
        ),
        "{said}"
    );
    assert!(
        said.lines()
            .last()
            .is_some_and(|line| line.starts_with("No problems")),
        "{said}"
    );
}

#[test]
fn doctor_and_build_list_the_same_features_off() {
    let site = demo("static");
    let doctor: Value = serde_json::from_slice(&khalid(&["--json"], site.path()).stdout).unwrap();
    let out = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--json", "--site"])
        .arg(site.path())
        .arg("build")
        .arg("--out")
        .arg(out.path())
        .output()
        .unwrap();
    let build: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(doctor["ok"], true, "{doctor}");
    assert_eq!(doctor["off"].as_array().map(Vec::len), Some(6), "{doctor}");
    assert_eq!(doctor["off"], build["off"], "{doctor}\n{build}");
}

#[test]
fn doctor_on_a_server_site_lists_nothing_off() {
    let site = demo("server");
    let output = khalid(&["--json"], site.path());
    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(body.get("off").is_none(), "{body}");
    let said = String::from_utf8_lossy(&khalid(&[], site.path()).stdout).into_owned();
    assert!(!said.contains("features that need a server"), "{said}");
}
