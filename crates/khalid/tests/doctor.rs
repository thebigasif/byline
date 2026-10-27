//! `khalid doctor`: every check of a site folder, each problem with what to
//! do, in human and `--json` output with exit codes (PRD KH-OPS-2, RULES.md
//! BL-CLI-1 to BL-CLI-3).

use std::process::{Command, Output};

use serde_json::Value;

/// The repository root, where the fixtures are.
const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

/// The checks that read the site's files, in the order doctor runs them.
const FILE_CHECKS: [&str; 3] = ["settings", "routes", "content"];

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
    let output = khalid(&["--site", site, "doctor", "--json"]);
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
fn the_demo_site_passes_every_file_check_and_exits_0() {
    let output = khalid(&["--site", "fixtures/demo-site", "doctor"]);
    let said = text(&output.stdout);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{said}{}",
        text(&output.stderr)
    );
    for heading in ["Settings: ok.", "Routes: ok.", "Content: ok."] {
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
    assert_eq!(&names[..3], &FILE_CHECKS, "{body}");
    for name in FILE_CHECKS {
        let found = check(&body, name);
        assert_eq!(found["status"], "ok", "{found}");
        assert!(found["summary"].as_str().unwrap().ends_with('.'), "{found}");
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
    let output = khalid(&["--site", &format!("fixtures/invalid/{name}"), "doctor"]);
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
