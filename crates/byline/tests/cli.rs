//! The command line contract: help and version (RULES.md BL-CLI-1 to BL-CLI-3).

use std::process::{Command, Output};

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

#[test]
fn help_says_what_byline_is() {
    let output = byline(&["--help"]);
    assert_eq!(output.status.code(), Some(0));
    let help = text(&output.stdout);
    assert!(help.contains("SEO defaults built in"), "{help}");
    assert!(help.contains("Usage: byline"), "{help}");
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
