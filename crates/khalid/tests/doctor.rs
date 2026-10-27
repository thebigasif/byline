//! `khalid doctor`: every check of a site folder, each problem with what to
//! do, in human and `--json` output with exit codes (PRD KH-OPS-2, RULES.md
//! BL-CLI-1 to BL-CLI-3).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;

/// The repository root, where the fixtures are.
const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

/// Every check, in the order doctor runs them.
const CHECKS: [&str; 8] = [
    "settings",
    "routes",
    "content",
    "database",
    "theme",
    "permissions",
    "address",
    "robots",
];

/// The checks that fetch over the network, skipped with `--offline`.
const ONLINE: [&str; 2] = ["address", "robots"];

/// Runs the binary from the repository root.
#[cfg(test)]
fn khalid(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_khalid"))
        .current_dir(ROOT)
        .args(args)
        .output()
        .unwrap()
}

/// Reads command output as text.
fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Runs doctor with `--json` on `site` and returns the exit code and the body.
#[cfg(test)]
fn doctor_json(site: &str) -> (Option<i32>, Value) {
    let output = khalid(&["--site", site, "doctor", "--offline", "--json"]);
    assert!(output.stderr.is_empty(), "{}", text(&output.stderr));
    let body = serde_json::from_slice(&output.stdout).unwrap();
    (output.status.code(), body)
}

/// The check named `name` in a doctor body.
#[cfg(test)]
fn check<'a>(body: &'a Value, name: &str) -> &'a Value {
    body["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|check| check["check"] == name)
        .unwrap_or_else(|| panic!("no {name} check in {body}"))
}

/// Every folder under `fixtures/invalid/` with its expected message.
#[cfg(test)]
fn invalid_sites() -> Vec<(String, String)> {
    let mut sites: Vec<(String, String)> = std::fs::read_dir(format!("{ROOT}/fixtures/invalid"))
        .unwrap()
        .map(Result::unwrap)
        .filter(|entry| entry.file_type().unwrap().is_dir())
        .map(|entry| {
            let name = entry.file_name().into_string().unwrap();
            let expected =
                std::fs::read_to_string(format!("{ROOT}/fixtures/invalid/{name}/expected.txt"))
                    .unwrap();
            (name, expected)
        })
        .collect();
    sites.sort();
    assert!(sites.len() >= 3, "only {} invalid sites", sites.len());
    sites
}

#[test]
fn the_demo_site_passes_every_check_and_exits_0() {
    let output = khalid(&["--site", "fixtures/demo-site", "doctor", "--offline"]);
    let said = text(&output.stdout);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{said}{}",
        text(&output.stderr)
    );
    for heading in [
        "Settings: ok.",
        "Routes: ok.",
        "Content: ok.",
        "Database: ok.",
        "Theme: ok.",
        "Permissions: ok.",
        "Public address: skipped.",
        "Host robots.txt: skipped.",
    ] {
        assert!(said.contains(heading), "no {heading:?} in:\n{said}");
    }
    assert!(
        said.lines()
            .last()
            .is_some_and(|line| line.starts_with("No problems")),
        "{said}"
    );
}

#[test]
fn the_demo_site_in_json_lists_each_check_in_order() {
    let (code, body) = doctor_json("fixtures/demo-site");
    assert_eq!(code, Some(0), "{body}");
    assert_eq!(body["ok"], true, "{body}");
    assert_eq!(body["problems"], 0, "{body}");
    let names: Vec<&str> = body["checks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|check| check["check"].as_str().unwrap())
        .collect();
    assert_eq!(names, CHECKS, "{body}");
    for name in CHECKS {
        let found = check(&body, name);
        let want = if ONLINE.contains(&name) {
            "skipped"
        } else {
            "ok"
        };
        assert_eq!(found["status"], want, "{found}");
        assert!(found["summary"].as_str().unwrap().ends_with('.'), "{found}");
    }
    for name in ONLINE {
        assert!(
            check(&body, name)["summary"]
                .as_str()
                .unwrap()
                .contains("--offline"),
            "{body}"
        );
    }
}

#[test]
fn every_invalid_site_shows_its_message_as_one_problem_and_exits_1() {
    for (name, expected) in invalid_sites() {
        let (code, body) = doctor_json(&format!("fixtures/invalid/{name}"));
        assert_eq!(code, Some(1), "{name}: {body}");
        assert_eq!(body["ok"], false, "{name}: {body}");
        assert_eq!(body["problems"], 1, "{name}: {body}");
        let problem = body["checks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|check| check["status"] == "problem")
            .unwrap();
        let said: Vec<&str> = ["what", "why", "fix"]
            .iter()
            .map(|part| problem["problem"][part].as_str().unwrap())
            .collect();
        let want: Vec<&str> = expected.lines().collect();
        assert_eq!(said, want, "{name}");
    }
}

#[test]
fn a_check_that_needs_a_failed_one_is_skipped_with_a_reason() {
    let (_, body) = doctor_json("fixtures/invalid/settings-unknown-field");
    assert_eq!(check(&body, "settings")["status"], "problem", "{body}");
    assert_eq!(check(&body, "routes")["status"], "ok", "{body}");
    let content = check(&body, "content");
    assert_eq!(content["status"], "skipped", "{content}");
    assert!(
        content["summary"].as_str().unwrap().contains("settings"),
        "{content}"
    );
}

#[test]
fn the_human_report_prints_each_problem_as_what_why_fix() {
    let name = "routes-no-slug";
    let (_, expected) = invalid_sites()
        .into_iter()
        .find(|(site, _)| site == name)
        .unwrap();
    let output = khalid(&[
        "--site",
        &format!("fixtures/invalid/{name}"),
        "doctor",
        "--offline",
    ]);
    let said = text(&output.stdout);
    assert_eq!(output.status.code(), Some(1), "{said}");
    assert!(said.contains("Routes: a problem."), "{said}");
    for line in expected.lines() {
        assert!(
            said.contains(&format!("  {line}\n")),
            "no {line:?} in:\n{said}"
        );
    }
    assert!(
        said.lines()
            .last()
            .is_some_and(|line| line.starts_with("1 problem")),
        "{said}"
    );
}

#[test]
fn doctor_without_a_site_folder_is_a_usage_error() {
    let output = khalid(&["doctor"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        text(&output.stderr).contains("needs a site folder"),
        "{}",
        text(&output.stderr)
    );
}

/// A copy of the demo site, so a test can break one thing in it.
#[cfg(test)]
fn demo_copy() -> TempDir {
    let mut demo = PathBuf::from(ROOT);
    demo.push("fixtures/demo-site");
    let temp = tempfile::tempdir().unwrap();
    copy_dir(&demo, temp.path());
    temp
}

#[cfg(test)]
fn copy_dir(from: &Path, into: &Path) {
    fs::create_dir_all(into).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = within(into, entry.file_name().to_str().unwrap());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// `name` inside `dir`.
#[cfg(test)]
fn within(dir: &Path, name: &str) -> PathBuf {
    let mut path = dir.to_path_buf();
    path.push(name);
    path
}

/// Writes `text` to `name` inside `site`, making its folder.
#[cfg(test)]
fn write(site: &Path, name: &str, text: &str) {
    let path = within(site, name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// Runs doctor with `--json` on `site` and returns the one problem it found.
#[cfg(test)]
fn one_problem(site: &Path, check_name: &str) -> Value {
    let (code, body) = doctor_json(site.to_str().unwrap());
    assert_eq!(code, Some(1), "{body}");
    assert_eq!(body["problems"], 1, "{body}");
    let found = check(&body, check_name);
    assert_eq!(found["status"], "problem", "{body}");
    found["problem"].clone()
}

#[test]
fn a_database_file_that_is_not_sqlite_is_a_problem() {
    let site = demo_copy();
    write(
        site.path(),
        "data/site.db",
        "These are notes, not a database.\n",
    );
    let problem = one_problem(site.path(), "database");
    assert!(
        problem["what"].as_str().unwrap().contains("site.db"),
        "{problem}"
    );
}

#[test]
fn a_site_with_no_database_yet_is_fine() {
    let site = demo_copy();
    let _ = fs::remove_dir_all(within(site.path(), "data"));
    let (code, body) = doctor_json(site.path().to_str().unwrap());
    assert_eq!(code, Some(0), "{body}");
    let database = check(&body, "database");
    assert!(
        database["summary"]
            .as_str()
            .unwrap()
            .contains("no database yet"),
        "{database}"
    );
    assert!(
        !within(site.path(), "data/site.db").exists(),
        "doctor made the database"
    );
}

#[test]
fn a_faint_theme_colour_is_a_problem_that_names_the_colour() {
    let site = demo_copy();
    write(
        site.path(),
        "theme/tokens.toml",
        "[colors]\nlink = \"#f0c040\"\n",
    );
    let problem = one_problem(site.path(), "theme");
    assert_eq!(
        problem["what"],
        "colors.link in theme/tokens.toml is too faint to read on the background."
    );
    assert!(
        problem["fix"].as_str().unwrap().contains("darker shade"),
        "{problem}"
    );
}

#[test]
fn a_partial_tokens_file_the_pages_accept_is_fine() {
    let site = demo_copy();
    write(
        site.path(),
        "theme/tokens.toml",
        "[colors]\nlink = \"#b3261e\"\n",
    );
    let (code, body) = doctor_json(site.path().to_str().unwrap());
    assert_eq!(code, Some(0), "{body}");
    assert_eq!(check(&body, "theme")["status"], "ok", "{body}");
}

#[test]
fn a_tokens_file_with_a_mistake_names_its_line() {
    let site = demo_copy();
    write(
        site.path(),
        "theme/tokens.toml",
        "[colors]\nlink = \"red-ish\"\n",
    );
    let problem = one_problem(site.path(), "theme");
    assert!(
        problem["what"].as_str().unwrap().contains("line 2"),
        "{problem}"
    );
}

#[cfg(unix)]
#[test]
fn a_folder_khalid_cannot_write_to_is_a_problem() {
    use std::os::unix::fs::PermissionsExt;

    let site = demo_copy();
    let content = within(site.path(), "content");
    fs::set_permissions(&content, fs::Permissions::from_mode(0o555)).unwrap();
    // A user who may write anywhere, such as root, cannot see this problem.
    let probe = tempfile::tempfile_in(&content);
    if probe.is_ok() {
        fs::set_permissions(&content, fs::Permissions::from_mode(0o755)).unwrap();
        return;
    }
    let (code, body) = doctor_json(site.path().to_str().unwrap());
    fs::set_permissions(&content, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(code, Some(1), "{body}");
    let problem = &check(&body, "permissions")["problem"];
    assert_eq!(problem["what"], "Khalid cannot write to content/.");
}

#[test]
fn doctor_leaves_the_site_folder_as_it_was() {
    let site = demo_copy();
    let before = listing(site.path());
    let (code, body) = doctor_json(site.path().to_str().unwrap());
    assert_eq!(code, Some(0), "{body}");
    assert_eq!(listing(site.path()), before);
}

/// Every path under `dir`, sorted.
#[cfg(test)]
fn listing(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            found.extend(listing(&path));
        }
        found.push(path);
    }
    found.sort();
    found
}
