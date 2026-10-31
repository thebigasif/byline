//! `khalid doctor` says whether photo and GIF search can work: each is off,
//! on with its key, or on without it, which is a problem (decision E14 in
//! `docs/decisions/2026-10-31-editor-library-pictures-and-crop.md`, plan
//! 4.6, KH-EDT-1, KH-OPS-2). Every key here is made up, and nothing is sent
//! to Unsplash or GIPHY.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

const UNSPLASH: &str = "KHALID_UNSPLASH_ACCESS_KEY";
const GIPHY: &str = "KHALID_GIPHY_API_KEY";

/// `name` inside `dir`.
#[cfg(test)]
fn within(dir: &Path, name: &str) -> PathBuf {
    let mut path = dir.to_path_buf();
    path.push(name);
    path
}

/// The demo site's settings with `media` added, in a fresh folder.
#[cfg(test)]
fn site_with(media: &str) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let settings = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/demo-site/site.toml"
    );
    let settings = format!("{}\n{media}", fs::read_to_string(settings).unwrap());
    fs::write(within(dir.path(), "site.toml"), settings).unwrap();
    fs::create_dir(within(dir.path(), "content")).unwrap();
    dir
}

/// Runs `khalid doctor --offline` with `args` on `site` with only the keys
/// in `keys` set, and returns the exit code and what it printed.
#[cfg(test)]
fn doctor(site: &Path, keys: &[(&str, &str)], args: &[&str]) -> (Option<i32>, String) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_khalid"));
    command
        .arg("--site")
        .arg(site)
        .args(["doctor", "--offline"])
        .args(args);
    command.env_remove(UNSPLASH).env_remove(GIPHY);
    command.envs(keys.iter().copied());
    let output = command.output().unwrap();
    let said = String::from_utf8_lossy(&output.stdout).into_owned();
    (output.status.code(), said)
}

/// The report for a person from the pictures check's title on.
#[cfg(test)]
fn human(site: &Path, keys: &[(&str, &str)]) -> (Option<i32>, String) {
    let (code, said) = doctor(site, keys, &[]);
    let Some((_, rest)) = said.split_once("Picture search:") else {
        panic!("no pictures check in:\n{said}");
    };
    (code, format!("Picture search:{rest}"))
}

/// The exit code, the pictures check from `--json`, and the whole output.
#[cfg(test)]
fn pictures(site: &Path, keys: &[(&str, &str)]) -> (Option<i32>, Value, String) {
    let (code, said) = doctor(site, keys, &["--json"]);
    let body: Value = serde_json::from_str(&said).unwrap();
    let check = body["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|check| check["check"] == "pictures")
        .unwrap_or_else(|| panic!("no pictures check in {body}"))
        .clone();
    (code, check, said)
}

#[test]
fn both_searches_off_pass_and_say_how_to_turn_them_on() {
    let site = site_with("");
    let (_, check, _) = pictures(site.path(), &[]);
    assert_eq!(check["status"], "ok", "{check}");
    assert_eq!(
        check["summary"],
        "Photo search on Unsplash and GIF search on GIPHY are off. \
         Turn them on with unsplash = true or gifs = true under [media] in site.toml.",
    );
}

#[test]
fn a_search_on_with_its_key_passes_without_showing_the_key() {
    let site = site_with("[media]\nunsplash = true\n");
    let (code, check, said) = pictures(site.path(), &[(UNSPLASH, "made-up-access-key")]);
    assert_eq!(check["status"], "ok", "{check}");
    assert_eq!(
        check["summary"],
        "Photo search on Unsplash is on, with its key. GIF search on GIPHY is off.",
    );
    assert!(!said.contains("made-up-access-key"), "{said}");
    assert_eq!(code, Some(0), "{said}");
}

#[test]
fn a_search_on_without_its_key_is_a_problem_naming_the_variable() {
    let site = site_with("[media]\nunsplash = true\ngifs = true\n");
    let (code, check, _) = pictures(site.path(), &[(UNSPLASH, "made-up-access-key")]);
    assert_eq!(code, Some(1));
    assert_eq!(check["status"], "problem", "{check}");
    let problem = &check["problem"];
    assert_eq!(
        problem["what"],
        "GIF search on GIPHY is on, but KHALID_GIPHY_API_KEY is not set."
    );
    assert_eq!(
        problem["why"],
        "Writers who look for a GIF see a message instead of GIFs."
    );
    assert_eq!(
        problem["fix"],
        "Set KHALID_GIPHY_API_KEY to the API key of a GIPHY app, \
         or set gifs = false under [media] in site.toml."
    );
}

#[test]
fn a_blank_key_counts_as_unset_in_the_report_for_a_person() {
    let site = site_with("[media]\nunsplash = true\n");
    let (code, block) = human(site.path(), &[(UNSPLASH, "  ")]);
    assert_eq!(code, Some(1));
    assert!(
        block.starts_with(
            "Picture search: a problem.\n  \
             Photo search on Unsplash is on, but KHALID_UNSPLASH_ACCESS_KEY is not set.\n"
        ),
        "{block}"
    );
}

#[test]
fn settings_that_do_not_load_skip_the_check() {
    let site = site_with("[media]\nunsplash = \"yes\"\n");
    let (_, check, _) = pictures(site.path(), &[]);
    assert_eq!(check["status"], "skipped", "{check}");
}
