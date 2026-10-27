//! What `khalid audit` says: the score, the issues by severity with their
//! fixes, the trend, and its messages when it cannot run.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::io;
use std::path::Path;

use khalid_audit::{AuditError, Check, Report, Run, Severity};
use khalid_core::UserMessage;
use serde::Serialize;

/// How many past scores the human report shows.
const TREND: usize = 5;

/// The JSON shape of `khalid audit --json`.
#[derive(Serialize)]
pub(crate) struct AuditBody<'a> {
    score: u8,
    pages: usize,
    errors: usize,
    warnings: usize,
    notices: usize,
    issues: Vec<IssueBody<'a>>,
    trend: &'a [Run],
    files: Files,
}

/// One issue in `--json`, with what the check is and how to fix it.
#[derive(Serialize)]
struct IssueBody<'a> {
    check: &'static str,
    severity: &'static str,
    category: &'static str,
    title: &'static str,
    url: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    detail: &'a str,
    fix: &'static str,
}

/// Where the audit wrote its files.
#[derive(Serialize)]
struct Files {
    issues: String,
    links: String,
    runs: String,
}

/// The JSON body for `audit`, whose files are in `folder`.
pub(crate) fn body<'a>(audit: &'a Report, trend: &'a [Run], folder: &Path) -> AuditBody<'a> {
    let file = |name: &str| format!("{}/{name}", folder.display());
    AuditBody {
        score: audit.score(),
        pages: audit.pages(),
        errors: audit.count(Severity::Error),
        warnings: audit.count(Severity::Warning),
        notices: audit.count(Severity::Notice),
        issues: audit
            .issues()
            .iter()
            .map(|issue| {
                let check = issue.check();
                IssueBody {
                    check: check.id(),
                    severity: check.severity().as_str(),
                    category: check.category().title(),
                    title: check.title(),
                    url: issue.url().as_str(),
                    detail: issue.detail(),
                    fix: check.fix(),
                }
            })
            .collect(),
        trend,
        files: Files {
            issues: file("issues.csv"),
            links: file("links.csv"),
            runs: file("runs.jsonl"),
        },
    }
}

/// The report for a person: the score, each check that found something,
/// grouped by severity with its count and fix, the last scores, and where
/// the full lists are.
pub(crate) fn human(audit: &Report, trend: &[Run], folder: &Path) -> String {
    let mut said = format!(
        "Audited {} pages. Health score: {} out of 100.\n",
        audit.pages(),
        audit.score()
    );
    let counts = [Severity::Error, Severity::Warning, Severity::Notice].map(|s| audit.count(s));
    let [errors, warnings, notices] = counts;
    let _ = writeln!(
        said,
        "{} {}, {} {}, {} {}.",
        errors,
        plural(errors, "error", "errors"),
        warnings,
        plural(warnings, "warning", "warnings"),
        notices,
        plural(notices, "notice", "notices")
    );
    // One page can fail a check more than once, such as with two broken
    // links, so the pages are counted, not the issues.
    let mut found: BTreeMap<(Severity, Check), BTreeSet<&str>> = BTreeMap::new();
    for issue in audit.issues() {
        found
            .entry((issue.check().severity(), issue.check()))
            .or_default()
            .insert(issue.url().as_str());
    }
    for ((severity, check), pages) in &found {
        let count = pages.len();
        let _ = writeln!(
            said,
            "{} in {}: {}, on {count} {}.\n  {}",
            capital(severity.as_str()),
            check.category().title(),
            check.title(),
            plural(count, "page", "pages"),
            check.fix()
        );
    }
    let scores: Vec<String> = trend
        .iter()
        .rev()
        .take(TREND)
        .rev()
        .map(|run| run.score.to_string())
        .collect();
    if scores.len() > 1 {
        let _ = writeln!(
            said,
            "The last scores, oldest first: {}.",
            scores.join(", ")
        );
    }
    let _ = writeln!(
        said,
        "Every issue with its page is in {0}/issues.csv, and the internal linking report in {0}/links.csv.",
        folder.display()
    );
    said
}

/// `one` when `count` is 1, else `many`.
fn plural(count: usize, one: &'static str, many: &'static str) -> &'static str {
    if count == 1 { one } else { many }
}

/// `word` with its first letter in capitals.
fn capital(word: &str) -> String {
    let mut letters = word.chars();
    letters
        .next()
        .map(|first| first.to_uppercase().chain(letters).collect())
        .unwrap_or_default()
}

/// The message for a built folder the audit cannot open.
pub(crate) fn of(error: &AuditError) -> UserMessage {
    UserMessage::new(
        format!("Khalid could not open the site it built for the audit: {error}."),
        "The audit reads the pages Khalid just built, so it cannot start without them.",
        "Run `khalid build` to see whether the build works, then run `khalid audit` again.",
    )
}

/// The message for a file in `data/audit/` that cannot be written.
pub(crate) fn write(file: &Path, error: &io::Error) -> UserMessage {
    UserMessage::new(
        format!("Khalid could not write {}: {error}.", file.display()),
        "The audit keeps its results and its trend in the data folder, so it stopped there.",
        "Check that the disk has room and the folder can be written to, then run `khalid audit` again.",
    )
}

/// The message for a fault inside Khalid.
pub(crate) fn internal() -> UserMessage {
    UserMessage::new(
        "Khalid could not finish the audit because of a fault inside Khalid.",
        "Nothing on the site was changed.",
        "Run `khalid audit` again, and report it if it happens again.",
    )
}
