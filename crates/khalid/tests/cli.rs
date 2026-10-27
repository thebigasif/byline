//! The command line contract: help, version, exit codes and messages, in both
//! human and `--json` output (RULES.md BL-CLI-1 to BL-CLI-3).

use std::process::{Command, Output};

use serde_json::Value;

/// Every command in docs/plan.md section 4.1 that is not built yet, with the
/// arguments it needs.
const COMMANDS: &[&[&str]] = &[
    &["publish", "why-internal-links-matter"],
    &["import", "substack", "export"],
    &["export"],
    &["backup"],
    &["audit"],
    &["update"],
    &["update", "--rollback"],
    &["pull"],
];

/// Runs the binary cargo built for this test.
#[cfg(test)]
fn khalid(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(args)
        .output()
        .unwrap()
}

/// Reads command output as text.
fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Checks one sentence of a message the way BL-CLI-3 asks.
#[cfg(test)]
fn assert_plain_sentence(sentence: &str, context: &str) {
    assert!(!sentence.is_empty(), "{context}: empty sentence");
    assert!(
        sentence.ends_with('.'),
        "{context}: {sentence:?} does not end with a full stop"
    );
    assert!(
        sentence.split_whitespace().count() < 25,
        "{context}: {sentence:?} is 25 words or more"
    );
    assert!(
        !sentence.contains('\u{2014}'),
        "{context}: {sentence:?} has an em dash"
    );
}

#[test]
fn help_lists_every_command() {
    let output = khalid(&["--help"]);
    assert_eq!(output.status.code(), Some(0));
    let help = text(&output.stdout);
    for command in [
        "init", "serve", "new", "lint", "publish", "theme", "import", "export", "mcp", "doctor",
        "backup", "audit", "update", "pull", "keys", "db",
    ] {
        assert!(
            help.contains(&format!("  {command} ")),
            "help does not list {command}:\n{help}"
        );
    }
    assert!(
        help.contains("--json"),
        "help does not mention --json:\n{help}"
    );
}

#[test]
fn theme_help_lists_sync_and_check() {
    let help = text(&khalid(&["theme", "--help"]).stdout);
    assert!(help.contains("  sync "), "{help}");
    assert!(help.contains("  check "), "{help}");
}

#[test]
fn no_arguments_prints_help_and_succeeds() {
    let output = khalid(&[]);
    assert_eq!(output.status.code(), Some(0));
    assert!(text(&output.stdout).contains("Usage: khalid"));
}

#[test]
fn version_prints_the_crate_version() {
    let output = khalid(&["--version"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        text(&output.stdout),
        format!("khalid {}\n", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn version_with_json_prints_json() {
    let output = khalid(&["--version", "--json"]);
    assert_eq!(output.status.code(), Some(0));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        json,
        serde_json::json!({ "name": "khalid", "version": env!("CARGO_PKG_VERSION") })
    );
}

#[test]
fn every_command_says_it_is_not_built_yet_and_exits_3() {
    for args in COMMANDS {
        let context = args.join(" ");
        let output = khalid(args);
        assert_eq!(output.status.code(), Some(3), "khalid {context}");
        assert!(output.stdout.is_empty(), "khalid {context} wrote to stdout");
        let stderr = text(&output.stderr);
        let lines: Vec<&str> = stderr.lines().collect();
        assert_eq!(
            lines.len(),
            3,
            "khalid {context}: want what, why, fix:\n{stderr}"
        );
        assert!(
            lines[0].contains("is not built yet"),
            "khalid {context}: {stderr}"
        );
        for line in lines {
            assert_plain_sentence(line, &context);
        }
    }
}

#[test]
fn every_command_accepts_json_and_prints_the_message_as_json() {
    for args in COMMANDS {
        let context = args.join(" ");
        let mut with_json = args.to_vec();
        with_json.push("--json");
        let output = khalid(&with_json);
        assert_eq!(output.status.code(), Some(3), "khalid {context} --json");
        assert!(
            output.stderr.is_empty(),
            "khalid {context} --json wrote to stderr"
        );
        let json: Value = serde_json::from_slice(&output.stdout).unwrap();
        for part in ["what", "why", "fix"] {
            let sentence = json["error"][part].as_str().unwrap();
            assert_plain_sentence(sentence, &format!("{context} --json, {part}"));
        }
    }
}

#[test]
fn json_works_before_the_command_too() {
    let output = khalid(&["--json", "audit"]);
    assert_eq!(output.status.code(), Some(3));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        json["error"]["what"]
            .as_str()
            .unwrap()
            .contains("khalid audit")
    );
}

#[test]
fn an_unknown_command_is_a_usage_error() {
    let output = khalid(&["launch"]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = text(&output.stderr);
    let lines: Vec<&str> = stderr.lines().collect();
    assert_eq!(lines.len(), 3, "{stderr}");
    assert!(lines[0].contains("launch"), "{stderr}");
    for line in lines {
        assert_plain_sentence(line, "launch");
    }
}

#[test]
fn a_usage_error_with_json_prints_json() {
    let output = khalid(&["launch", "--json"]);
    assert_eq!(output.status.code(), Some(2));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json["error"]["what"].as_str().unwrap().contains("launch"));
    assert!(
        json["error"]["fix"]
            .as_str()
            .unwrap()
            .contains("khalid --help")
    );
}

#[test]
fn publish_refuses_a_slug_that_breaks_the_rules() {
    let output = khalid(&["publish", "Why Links Matter"]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = text(&output.stderr);
    assert!(
        stderr.contains("only a to z, 0 to 9 and hyphens"),
        "{stderr}"
    );
}

#[test]
fn import_lists_the_sources_it_knows() {
    let output = khalid(&["import", "blogger"]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = text(&output.stderr);
    assert!(stderr.contains("wordpress"), "{stderr}");
}

/// The repository root, where the fixture paths in expected messages start.
const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

/// Runs the binary from the repository root.
#[cfg(test)]
fn khalid_at_root(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_khalid"))
        .current_dir(ROOT)
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn every_invalid_site_prints_its_expected_message() {
    let mut checked = 0;
    for entry in std::fs::read_dir(format!("{ROOT}/fixtures/invalid")).unwrap() {
        let entry = entry.unwrap();
        if !entry.file_type().unwrap().is_dir() {
            continue;
        }
        let name = entry.file_name().into_string().unwrap();
        let expected =
            std::fs::read_to_string(format!("{ROOT}/fixtures/invalid/{name}/expected.txt"))
                .unwrap();
        let site = format!("fixtures/invalid/{name}");
        let output = khalid_at_root(&["--site", &site, "lint"]);
        assert_eq!(output.status.code(), Some(1), "{name}");
        assert_eq!(text(&output.stderr), expected, "{name}");
        for line in expected.lines() {
            assert_plain_sentence(line, &name);
        }
        checked += 1;
    }
    assert!(checked >= 3, "only {checked} invalid sites were checked");
}

#[test]
fn a_site_with_valid_settings_goes_on_to_the_command() {
    let output = khalid_at_root(&["--site", "fixtures/demo-site", "lint"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    assert!(
        text(&output.stdout).contains("0 blocks and"),
        "the demo site has no blocks"
    );
}

#[test]
fn a_settings_mistake_with_json_prints_json() {
    let output = khalid_at_root(&[
        "--site",
        "fixtures/invalid/settings-unknown-mode",
        "lint",
        "--json",
    ]);
    assert_eq!(output.status.code(), Some(1));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        json["error"]["what"]
            .as_str()
            .unwrap()
            .contains("line 6, `mode`")
    );
    assert!(
        json["error"]["fix"]
            .as_str()
            .unwrap()
            .contains("`server`, `static`, `headless`")
    );
}

#[test]
fn help_lists_the_site_flag() {
    let help = text(&khalid(&["--help"]).stdout);
    assert!(help.contains("--site <FOLDER>"), "{help}");
}
