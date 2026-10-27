//! `khalid agents`: the guide for AI agents, built into the binary, in
//! both output modes (KH-API-5, RULES.md BL-CLI-1).

use std::process::{Command, Output};

use serde_json::Value;

#[cfg(test)]
fn khalid(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(args)
        .output()
        .unwrap()
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
