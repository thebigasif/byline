//! The command line contract: help, version, exit codes and messages, in both
//! human and `--json` output (RULES.md BL-CLI-1 to BL-CLI-3).

use std::process::{Command, Output};

use serde_json::Value;

/// Every command in docs/plan.md section 4.1, with the arguments it needs.
const COMMANDS: &[&[&str]] = &[
    &["init"],
    &["serve"],
    &["new", "Why internal links matter"],
    &["lint"],
    &["lint", "content/en/posts/choosing-a-crm.md"],
    &["publish", "why-internal-links-matter"],
    &["theme", "sync", "https://serpdad.com"],
    &["theme", "check"],
    &["import", "wordpress"],
    &["export"],
    &["mcp"],
    &["doctor"],
    &["backup"],
    &["audit"],
    &["update"],
    &["update", "--rollback"],
    &["pull"],
    &["keys"],
];

/// Runs the binary cargo built for this test.
#[cfg(test)]
fn byline(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_byline"))
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
    let output = byline(&["--help"]);
    assert_eq!(output.status.code(), Some(0));
    let help = text(&output.stdout);
    for command in [
        "init", "serve", "new", "lint", "publish", "theme", "import", "export", "mcp", "doctor",
        "backup", "audit", "update", "pull", "keys",
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
    let help = text(&byline(&["theme", "--help"]).stdout);
    assert!(help.contains("  sync "), "{help}");
    assert!(help.contains("  check "), "{help}");
}

#[test]
fn no_arguments_prints_help_and_succeeds() {
    let output = byline(&[]);
    assert_eq!(output.status.code(), Some(0));
    assert!(text(&output.stdout).contains("Usage: byline"));
}

#[test]
fn version_prints_the_crate_version() {
    let output = byline(&["--version"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        text(&output.stdout),
        format!("byline {}\n", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn version_with_json_prints_json() {
    let output = byline(&["--version", "--json"]);
    assert_eq!(output.status.code(), Some(0));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        json,
        serde_json::json!({ "name": "byline", "version": env!("CARGO_PKG_VERSION") })
    );
}

#[test]
fn every_command_says_it_is_not_built_yet_and_exits_3() {
    for args in COMMANDS {
        let context = args.join(" ");
        let output = byline(args);
        assert_eq!(output.status.code(), Some(3), "byline {context}");
        assert!(output.stdout.is_empty(), "byline {context} wrote to stdout");
        let stderr = text(&output.stderr);
        let lines: Vec<&str> = stderr.lines().collect();
        assert_eq!(
            lines.len(),
            3,
            "byline {context}: want what, why, fix:\n{stderr}"
        );
        assert!(
            lines[0].contains("is not built yet"),
            "byline {context}: {stderr}"
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
        let output = byline(&with_json);
        assert_eq!(output.status.code(), Some(3), "byline {context} --json");
        assert!(
            output.stderr.is_empty(),
            "byline {context} --json wrote to stderr"
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
    let output = byline(&["--json", "lint"]);
    assert_eq!(output.status.code(), Some(3));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        json["error"]["what"]
            .as_str()
            .unwrap()
            .contains("byline lint")
    );
}

#[test]
fn an_unknown_command_is_a_usage_error() {
    let output = byline(&["launch"]);
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
    let output = byline(&["launch", "--json"]);
    assert_eq!(output.status.code(), Some(2));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json["error"]["what"].as_str().unwrap().contains("launch"));
    assert!(
        json["error"]["fix"]
            .as_str()
            .unwrap()
            .contains("byline --help")
    );
}

#[test]
fn publish_refuses_a_slug_that_breaks_the_rules() {
    let output = byline(&["publish", "Why Links Matter"]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = text(&output.stderr);
    assert!(
        stderr.contains("only lowercase letters a to z, digits and hyphens"),
        "{stderr}"
    );
}

#[test]
fn import_lists_the_sources_it_knows() {
    let output = byline(&["import", "blogger"]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = text(&output.stderr);
    assert!(stderr.contains("wordpress"), "{stderr}");
}
