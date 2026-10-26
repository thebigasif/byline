//! `khalid lint`: every rule over the site.

use std::fmt::Write as _;
use std::path::Path;

use khalid_content::{ContentTree, read_content};
use khalid_core::{Config, Environment, SystemClock};
use khalid_lint::{Finding, LintConfig, Linter, Report, Severity};

use crate::cli::Output;
use crate::exit::Exit;
use crate::{config_message, content_message, lint_input, lint_message, report};

/// Lints every post in `site` and exits with 1 when any finding blocks
/// publishing.
pub(crate) fn run(site: &Path, output: Output) -> Exit {
    let config = match Config::load(site, &Environment::from_process()) {
        Ok(config) => config,
        Err(error) => return report::failure(output, &config_message::of(&error), Exit::Failure),
    };
    let content = match ContentTree::walk(site).and_then(|tree| read_content(&tree)) {
        Ok(content) => content,
        Err(error) => {
            return report::failure(output, &content_message::of(site, &error), Exit::Failure);
        }
    };
    let linter = match Linter::new(&LintConfig::default(), &SystemClock) {
        Ok(linter) => linter,
        Err(error) => return report::failure(output, &lint_message::of(&error), Exit::Internal),
    };
    let lint_site = lint_input::lint_site(&content, &config);
    let report = linter.lint_site(&lint_site);
    let exit = if report.blocks() {
        Exit::Failure
    } else {
        Exit::Success
    };
    match output {
        Output::Json => report::json(&report, exit),
        Output::Human => report::text(&sentences(&report), exit),
    }
}

/// Every finding as sentences, then a count of blocks and warnings.
fn sentences(report: &Report) -> String {
    let mut text = String::new();
    for finding in report.findings() {
        write_finding(&mut text, finding);
    }
    let blocks = report
        .findings()
        .iter()
        .filter(|f| f.severity() == Severity::Block);
    let blocks = blocks.count();
    let warnings = report.findings().len() - blocks;
    let _ = writeln!(
        text,
        "{} and {}.",
        count(blocks, "block"),
        count(warnings, "warning")
    );
    text
}

/// One finding: where it is and how much it matters, then its three
/// sentences, then its rule and guide.
fn write_finding(text: &mut String, finding: &Finding) {
    let at = finding.location();
    let kind = match finding.severity() {
        Severity::Block => "blocks publishing",
        _ => "warning",
    };
    let message = finding.message();
    let _ = writeln!(text, "{}/{}: {kind}", at.locale, at.slug);
    for line in [message.what(), message.why(), message.fix()] {
        let _ = writeln!(text, "  {line}");
    }
    let _ = writeln!(
        text,
        "  Rule {}, guide {}.\n",
        finding.rule(),
        message.guide()
    );
}

/// `n` and `noun`, with an `s` unless `n` is one.
fn count(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("1 {noun}")
    } else {
        format!("{n} {noun}s")
    }
}
