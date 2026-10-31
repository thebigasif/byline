//! What `khalid audit` says: the score, the issues by severity with their
//! fixes, the trend, and its messages when it cannot run.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::io;
use std::path::Path;

use khalid_audit::{AuditError, Check, PageWeight, Report, Run, Severity, WeightReport};
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
    #[serde(skip_serializing_if = "Option::is_none")]
    heaviest: Option<WeightBody<'a>>,
    trend: &'a [Run],
    files: Files,
}

/// The heaviest page in `--json`, every weight in bytes.
#[derive(Serialize)]
struct WeightBody<'a> {
    url: &'a str,
    total: usize,
    html: usize,
    styles: usize,
    scripts: usize,
    images: usize,
    other: usize,
    unweighed: usize,
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
    why: &'static str,
    fix: &'static str,
}

/// Where the audit wrote its files.
#[derive(Serialize)]
struct Files {
    issues: String,
    links: String,
    weights: String,
    runs: String,
}

/// The JSON body for `audit` with its page `weights`, whose files are in
/// `folder`.
pub(crate) fn body<'a>(
    (audit, weights): (&'a Report, &'a WeightReport),
    trend: &'a [Run],
    folder: &Path,
) -> AuditBody<'a> {
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
                    why: check.why(),
                    fix: check.fix(),
                }
            })
            .collect(),
        heaviest: weights.heaviest().map(|page| WeightBody {
            url: page.url().as_str(),
            total: page.total(),
            html: page.html(),
            styles: page.styles(),
            scripts: page.scripts(),
            images: page.images(),
            other: page.other(),
            unweighed: page.unweighed(),
        }),
        trend,
        files: Files {
            issues: file("issues.csv"),
            links: file("links.csv"),
            weights: file("weights.csv"),
            runs: file("runs.jsonl"),
        },
    }
}

/// The report for a person: the score, each check that found something,
/// grouped by severity with its count and fix, the last scores, the
/// heaviest page of `weights`, and where the full lists are.
pub(crate) fn human(
    (audit, weights): (&Report, &WeightReport),
    trend: &[Run],
    folder: &Path,
) -> String {
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
            "{} in {}: {}, on {count} {}.\n  {}\n  {}",
            capital(severity.as_str()),
            check.category().title(),
            check.title(),
            plural(count, "page", "pages"),
            check.why(),
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
    if let Some(page) = weights.heaviest() {
        heaviest(&mut said, page);
    }
    let _ = writeln!(
        said,
        "Every issue with its page is in {0}/issues.csv, the internal linking report in {0}/links.csv, and every page's weight in {0}/weights.csv.",
        folder.display()
    );
    said
}

/// Says what the heaviest `page` weighs, and of what.
fn heaviest(said: &mut String, page: &PageWeight) {
    let _ = writeln!(
        said,
        "The heaviest page is {}: {} in all, {} of HTML, {} of styles, {} of scripts, {} of images and {} of other files.",
        page.url().as_str(),
        size(page.total()),
        size(page.html()),
        size(page.styles()),
        size(page.scripts()),
        size(page.images()),
        size(page.other()),
    );
    let _ = writeln!(
        said,
        "  A lighter page opens sooner for readers and costs search engines and AI tools less to fetch.\n  To lighten it, start with its largest part, such as images saved smaller."
    );
    let unweighed = page.unweighed();
    if unweighed > 0 {
        let _ = writeln!(
            said,
            "  It loads {unweighed} {} on other sites or broken, which the audit does not download, so the real weight is higher.",
            plural(unweighed, "file", "files")
        );
    }
}

/// `bytes` for a person: whole bytes below a kilobyte (KB, 1,000 bytes),
/// then KB or MB with one decimal, rounded down.
fn size(bytes: usize) -> String {
    let tenths = |unit: usize| format!("{}.{}", bytes / unit, bytes % unit * 10 / unit);
    match bytes {
        ..1_000 => format!("{bytes} {}", plural(bytes, "byte", "bytes")),
        1_000..1_000_000 => format!("{} KB", tenths(1_000)),
        _ => format!("{} MB", tenths(1_000_000)),
    }
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

#[cfg(test)]
mod tests {
    use super::size;

    #[test]
    fn a_size_reads_in_bytes_then_kilobytes_then_megabytes() {
        assert_eq!(size(1), "1 byte");
        assert_eq!(size(999), "999 bytes");
        assert_eq!(size(1_000), "1.0 KB");
        assert_eq!(size(12_345), "12.3 KB");
        assert_eq!(size(2_590_000), "2.5 MB");
    }
}
