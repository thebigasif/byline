//! `khalid lint`: every rule over the site, or the post rules over one post.

use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::Path;

use khalid_content::{ContentTree, read_content};
use khalid_core::{Environment, SitePath, SystemClock, UserMessage};
use khalid_lint::{Finding, LintConfig, LintPost, Linter, Report, Score, Severity};
use khalid_routes::Routes;
use serde::Serialize;

use crate::cli::Output;
use crate::exit::Exit;
use crate::lint_fix::Repair;
use crate::lint_input::LintInput;
use crate::{
    content_message, lint_fix, lint_input, lint_message, report, routes_message, site_briefs,
    site_config,
};

/// Whether `khalid lint` only reports, or makes the safe repairs first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    /// Report the findings and change no file.
    Check,
    /// Make the safe repairs in the files, then report what is left.
    Fix,
}

/// A run's report, with the repairs `--fix` made and, for a whole site, each
/// post's score. In JSON it is the report with a `fixed` list and a
/// `scores` list beside `blocks` and `findings`; a run on one post has the
/// report's own `score` instead.
#[derive(Serialize)]
struct Outcome {
    #[serde(skip_serializing_if = "Option::is_none")]
    fixed: Option<Vec<Repair>>,
    #[serde(flatten)]
    report: Report,
    #[serde(skip_serializing_if = "Option::is_none")]
    scores: Option<Vec<PostScore>>,
}

/// One post's score out of 100, in JSON `{"locale": "en", "slug": "...",
/// "value": 85, "lost": [...]}`.
#[derive(Serialize)]
struct PostScore {
    locale: String,
    slug: String,
    #[serde(flatten)]
    score: Score,
}

/// Lints the site in `site`, or only the post file `post` inside it, and
/// exits with 1 when any finding blocks publishing. With [`Mode::Fix`] the
/// exit code is for what is left after the repairs.
pub(crate) fn run(site: &Path, post: Option<&Path>, mode: Mode, output: Output) -> Exit {
    let outcome = match check(site, post, mode) {
        Ok(outcome) => outcome,
        Err((message, exit)) => return report::failure(output, &message, exit),
    };
    let exit = if outcome.report.blocks() {
        Exit::Failure
    } else {
        Exit::Success
    };
    match output {
        Output::Json => report::json(&outcome, exit),
        Output::Human => {
            let mut text = sentences(&outcome.report, outcome.fixed.as_deref());
            write_scores(&mut text, &outcome);
            report::text(&text, exit)
        }
    }
}

/// Reads the site and lints it, and with [`Mode::Fix`] repairs what it can
/// and lints it again.
fn check(site: &Path, post: Option<&Path>, mode: Mode) -> Result<Outcome, (UserMessage, Exit)> {
    let config = site_config::load(site, &Environment::from_process())
        .map_err(|message| (message, Exit::Failure))?;
    let content = ContentTree::walk(site)
        .and_then(|tree| read_content(&tree))
        .map_err(|error| (content_message::of(site, &error), Exit::Failure))?;
    let settings = lint_settings(site).map_err(|message| (message, Exit::Failure))?;
    let linter = Linter::new(&settings, &SystemClock)
        .map_err(|error| (lint_message::of(&error), Exit::Internal))?;
    let routes = Routes::load(site).map_err(|error| (routes_message::of(&error), Exit::Failure))?;
    let briefs = site_briefs::load(site).map_err(|message| (message, Exit::Failure))?;
    let input = LintInput::new(&content, &config, routes).with_briefs(briefs);
    let (lint_site, documents) = lint_input::lint_site(&content, &config, &input);
    let (report, scores) = match post {
        None => {
            let report = linter.lint_site(&lint_site);
            let scores = scores(&report, &lint_site.posts);
            (report, Some(scores))
        }
        Some(wanted) => {
            let index = documents.iter().position(|d| d.path.ends_with(wanted));
            match index.and_then(|i| lint_site.posts.get(i)) {
                Some(found) => (linter.lint_post(found, &lint_site), None),
                None => return Err((no_such_post(wanted), Exit::Failure)),
            }
        }
    };
    let checked = Outcome {
        fixed: None,
        report,
        scores,
    };
    if mode == Mode::Check {
        return Ok(checked);
    }
    let findings = checked.report.findings();
    let fixed = lint_fix::repair(site, &lint_site, &documents, findings, &config)
        .map_err(|message| (message, Exit::Failure))?;
    let outcome = if fixed.is_empty() {
        checked
    } else {
        check(site, post, Mode::Check)?
    };
    Ok(Outcome {
        fixed: Some(fixed),
        ..outcome
    })
}

/// The score of each of `posts`, in their order.
fn scores(report: &Report, posts: &[LintPost]) -> Vec<PostScore> {
    report
        .scores(posts)
        .into_iter()
        .map(|(post, score)| PostScore {
            locale: post.locale.to_string(),
            slug: post.slug.to_string(),
            score,
        })
        .collect()
}

/// The settings in the site's `lint.toml`, or the defaults when it has none.
pub(crate) fn lint_settings(site: &Path) -> Result<LintConfig, UserMessage> {
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

/// The repairs by file, every finding as sentences, then a count of blocks
/// and warnings, and of repairs when `--fix` ran.
fn sentences(report: &Report, fixed: Option<&[Repair]>) -> String {
    let mut text = String::new();
    let mut file = None;
    for repair in fixed.unwrap_or_default() {
        if file != Some(&repair.path) {
            let gap = if file.is_some() { "\n" } else { "" };
            let _ = writeln!(text, "{gap}Fixed {}", repair.path);
            file = Some(&repair.path);
        }
        let _ = writeln!(text, "  {}", repair.change);
    }
    if file.is_some() {
        text.push('\n');
    }
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
    match fixed {
        Some([]) => text.push_str("Nothing needed a safe fix.\n"),
        Some(repairs) => {
            let _ = writeln!(text, "Fixed {}.", count(repairs.len(), "problem"));
        }
        None => {}
    }
    text
}

/// The score out of 100 of the one post checked, or of each post of the
/// site that lost points, in site order.
fn write_scores(text: &mut String, outcome: &Outcome) {
    if let Some(score) = outcome.report.post_score() {
        let _ = writeln!(text, "Score: {} out of 100.", score.value());
    }
    let Some(scores) = &outcome.scores else {
        return;
    };
    let mut below = scores.iter().filter(|post| post.score.value() < 100);
    match below.next() {
        None => text.push_str("Every post scores 100 out of 100.\n"),
        Some(first) => {
            text.push_str("Scores out of 100:\n");
            for post in std::iter::once(first).chain(below) {
                let value = post.score.value();
                let _ = writeln!(text, "  {}/{}: {value}", post.locale, post.slug);
            }
        }
    }
}

/// One finding as its post, its severity and its three sentences, then its
/// subject, suggestions, rule and guide, as `khalid lint` and a refused
/// `khalid build` both show it.
pub(crate) fn write_finding(text: &mut String, finding: &Finding) {
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
