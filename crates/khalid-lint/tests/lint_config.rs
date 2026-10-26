//! Severities from `lint.toml`, and mistakes in it named by line and column.

use khalid_lint::{LintConfig, LintError, RuleId, Severity, Thresholds};

#[test]
fn lint_toml_sets_a_severity_per_rule() {
    let config = LintConfig::parse("[rules]\nno-date = \"warn\"\nno-author = \"off\"\n").unwrap();
    assert_eq!(
        config.severity(RuleId::new("no-date")),
        Some(Severity::Warn)
    );
    assert_eq!(
        config.severity(RuleId::new("no-author")),
        Some(Severity::Off)
    );
    assert_eq!(
        config.severity(RuleId::new("no-title")),
        None,
        "a rule the file does not name keeps its default"
    );
}

#[test]
fn an_empty_lint_toml_keeps_every_default() {
    assert_eq!(LintConfig::parse("").unwrap(), LintConfig::default());
}

#[test]
fn an_unknown_rule_names_its_line_and_column() {
    let error =
        LintConfig::parse("[rules]\nno-date = \"warn\"\n  no-dates = \"off\"\n").unwrap_err();
    assert!(
        matches!(&error, LintError::UnknownRule { rule, line: 3, column: 3 } if rule == "no-dates"),
        "{error:?}"
    );
    assert_eq!(
        error.to_string(),
        "lint.toml line 3, column 3: there is no rule called \"no-dates\""
    );
}

#[test]
fn an_unknown_severity_names_its_line() {
    let error = LintConfig::parse("[rules]\nno-date = \"error\"\n").unwrap_err();
    assert!(
        matches!(error, LintError::Syntax { line: 2, .. }),
        "{error:?}"
    );
}

#[test]
fn an_unknown_section_names_its_line() {
    let error = LintConfig::parse("[rules]\n\n[rulez]\n").unwrap_err();
    assert!(
        matches!(error, LintError::Syntax { line: 3, .. }),
        "{error:?}"
    );
}

#[test]
fn text_that_is_not_toml_names_its_line() {
    let error = LintConfig::parse("[rules]\nno-date = warn\n").unwrap_err();
    assert!(
        matches!(error, LintError::Syntax { line: 2, .. }),
        "{error:?}"
    );
}

#[test]
fn lint_toml_tunes_a_threshold() {
    let config = LintConfig::parse("[thresholds]\nsame-link-text-max = 5\n").unwrap();
    assert_eq!(config.thresholds().same_link_text_max, 5);
    assert_eq!(LintConfig::default().thresholds(), Thresholds::default());
}

#[test]
fn an_unknown_threshold_names_its_line() {
    let error = LintConfig::parse("[thresholds]\n\nsame-link-max = 5\n").unwrap_err();
    assert!(
        matches!(error, LintError::Syntax { line: 3, .. }),
        "{error:?}"
    );
}

#[test]
fn a_negative_threshold_names_its_line() {
    let error = LintConfig::parse("[thresholds]\nsame-link-text-max = -1\n").unwrap_err();
    assert!(
        matches!(error, LintError::Syntax { line: 2, .. }),
        "{error:?}"
    );
}
