//! `khalid lint`: every rule over the site, or the post rules over one post.

use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::Path;

use khalid_content::{ContentTree, read_content};
use khalid_core::{Config, Environment, SitePath, SystemClock, UserMessage};
use khalid_lint::{Finding, LintConfig, Linter, Report, Severity};

use crate::cli::Output;
use crate::exit::Exit;
use crate::{config_message, content_message, lint_input, lint_message, report};

/// Lints the site in `site`, or only the post file `post` inside it, and
/// exits with 1 when any finding blocks publishing.
pub(crate) fn run(site: &Path, post: Option<&Path>, output: Output) -> Exit {
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
    let linter = match lint_settings(site).map(|settings| Linter::new(&settings, &SystemClock)) {
        Ok(Ok(linter)) => linter,
        Ok(Err(error)) => {
            return report::failure(output, &lint_message::of(&error), Exit::Internal);
        }
        Err(message) => return report::failure(output, &message, Exit::Failure),
    };
    let (lint_site, files) = lint_input::lint_site(&content, &config);
    let report = match post {
        None => linter.lint_site(&lint_site),
        Some(wanted) => {
            let index = files.iter().position(|file| file.ends_with(wanted));
            match index.and_then(|i| lint_site.posts.get(i)) {
                Some(found) => linter.lint_post(found, &lint_site),
                None => return report::failure(output, &no_such_post(wanted), Exit::Failure),
            }
        }
    };
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

/// The settings in the site's `lint.toml`, or the defaults when it has none.
fn lint_settings(site: &Path) -> Result<LintConfig, UserMessage> {
    let path = SitePath::within(site, "lint.toml").map_err(|_| lint_message::unreadable())?;
    match fs::read_to_string(path.as_path()) {
        Ok(text) => LintConfig::parse(&text).map_err(|error| lint_message::of(&error)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(LintConfig::default()),
        Err(_) => Err(lint_message::unreadable()),
    }
}

/// The message for a post path that matches no post file.
fn no_such_post(wanted: &Path) -> UserMessage {
    UserMessage::new(
        format!("Khalid found no post at `{}`.", wanted.display()),
        "Khalid checks one post only when the path names a post file in the site.",
        "Give the path from the site folder, such as `content/en/posts/my-post.md`.",
    )
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
    if let Some(subject) = &at.subject {
        let _ = writeln!(text, "  About: {subject}");
    }
    if !finding.suggestions().is_empty() {
        let _ = writeln!(text, "  Try: {}", finding.suggestions().join(", "));
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
