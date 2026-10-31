//! `khalid agents`: the guide for AI agents, built into the binary, in
//! both output modes (KH-API-5, RULES.md BL-CLI-1), and the promise it
//! makes: every API operation a key may call is an MCP tool.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::{Value, json};

#[cfg(test)]
fn khalid(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(args)
        .output()
        .unwrap()
}

#[cfg(test)]
fn copy_dir(from: &Path, into: &Path) {
    fs::create_dir_all(into).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let mut target = into.to_path_buf();
        target.push(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// The file at `relative` from the root of the repository.
#[cfg(test)]
fn repository(relative: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../..");
    path.push(relative);
    path
}

/// The names of the tools `khalid mcp` lists for a copy of the demo site.
#[cfg(test)]
fn tool_names() -> Vec<String> {
    let site = tempfile::tempdir().unwrap();
    copy_dir(&repository("fixtures/demo-site"), site.path());
    let made = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .arg("--site")
        .arg(site.path())
        .args([
            "keys",
            "create",
            "--json",
            "--name",
            "Claude",
            "--scope",
            "posts:write",
        ])
        .output()
        .unwrap();
    assert_eq!(made.status.code(), Some(0), "{made:?}");
    let made: Value = serde_json::from_slice(&made.stdout).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .arg("--site")
        .arg(site.path())
        .arg("mcp")
        .env("KHALID_KEY", made["key"].as_str().unwrap())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let request = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" });
    writeln!(input, "{request}").unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    drop(input);
    let finished = child.wait_with_output().unwrap();
    assert_eq!(finished.status.code(), Some(0), "{finished:?}");
    let answer: Value = serde_json::from_str(&line).unwrap();
    let tools = answer["result"]["tools"].as_array().unwrap();
    tools
        .iter()
        .map(|tool| tool["name"].as_str().unwrap().to_owned())
        .collect()
}

/// Brief G26's done line: the test fails when an operation in the
/// document `khalid serve` answers has no MCP tool. Operations no key may
/// call, such as signing in, are for a person in a browser.
#[test]
fn every_operation_a_key_may_call_is_an_mcp_tool() {
    let document = fs::read_to_string(repository("sdk/typescript/openapi.json")).unwrap();
    let document: Value = serde_json::from_str(&document).unwrap();
    let tools = tool_names();
    let mut missing = Vec::new();
    let mut checked = 0;
    for item in document["paths"].as_object().unwrap().values() {
        for operation in item.as_object().unwrap().values() {
            let schemes = operation["security"].to_string();
            if !schemes.contains("admin_key") && !schemes.contains("content_key") {
                continue;
            }
            checked += 1;
            let name = operation["operationId"].as_str().unwrap();
            if !tools.iter().any(|tool| tool == name) {
                missing.push(name);
            }
        }
    }
    assert!(checked > 50, "only {checked} operations were read");
    assert!(
        missing.is_empty(),
        "operations with no MCP tool: {missing:?}"
    );
}

#[test]
fn the_guide_prints_without_a_site_and_explains_how_to_connect() {
    let output = khalid(&["agents"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stderr, b"");
    let guide = String::from_utf8(output.stdout).unwrap();
    assert!(guide.starts_with("# Khalid for AI agents\n"), "{guide}");
    for needed in [
        "KHALID_KEY",
        "keys create",
        "--json",
        "updated_at",
        "drafts/",
    ] {
        assert!(guide.contains(needed), "the guide never mentions {needed}");
    }
    assert!(!guide.contains('\u{2014}'), "the guide has an em dash");
}

#[test]
fn with_json_the_guide_is_one_value() {
    let human = khalid(&["agents"]);
    let json = khalid(&["agents", "--json"]);
    assert_eq!(json.status.code(), Some(0));
    let value: Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(
        value["guide"].as_str().map(str::as_bytes),
        Some(&human.stdout[..])
    );
}
