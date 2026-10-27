//! `khalid --site <folder> doctor`: run every check on a site folder and
//! print each problem with what to do. It exits with 1 when any check finds a
//! problem. It changes nothing: the permission check writes a hidden empty
//! file and removes it at once.

use std::fmt::Write as _;
use std::path::Path;

use khalid_core::UserMessage;
use serde::Serialize;

use crate::cli::{Cli, Network, Output};
use crate::doctor_check::{Check, Finding, Outcome, Status};
use crate::exit::Exit;
use crate::{doctor_address, doctor_files, doctor_folder, doctor_robots, report, static_mode};

/// The JSON shape of a doctor run.
#[derive(Serialize)]
struct DoctorBody<'a> {
    /// The site folder, as given.
    site: String,
    /// Whether no check found a problem.
    ok: bool,
    /// How many checks found a problem.
    problems: usize,
    /// Every check, in the order it ran.
    checks: Vec<CheckBody<'a>>,
    /// The features a static site has off, each with what, why and fix.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    off: Vec<static_mode::Off>,
}

/// The JSON shape of one check. `summary` is there when it passed or was
/// skipped, `problem` when it found one.
#[derive(Serialize)]
struct CheckBody<'a> {
    check: Check,
    status: Status,
    #[serde(skip_serializing_if = "Option::is_none")]
    summary: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    problem: Option<&'a UserMessage>,
}

impl<'a> From<&'a Finding> for CheckBody<'a> {
    fn from(finding: &'a Finding) -> Self {
        let (summary, problem) = match &finding.outcome {
            Outcome::Ok(said) | Outcome::Skipped(said) => (Some(said.as_str()), None),
            Outcome::Problem(message) => (None, Some(message)),
        };
        Self {
            check: finding.check,
            status: finding.status(),
            summary,
            problem,
        }
    }
}

/// Runs every check on the `site` folder, then fetches its public address
/// and the host's `robots.txt` unless `network` says to stay offline, and
/// reports them all, with the
/// features a static site has off.
pub(crate) fn run(site: &Path, network: Network, cli: &Cli) -> Exit {
    let (mut findings, config) = doctor_files::check(site);
    findings.extend(doctor_folder::check(site));
    findings.push(doctor_address::check(config.as_ref(), network, cli));
    findings.push(doctor_robots::check(config.as_ref(), network, cli));
    let off = config
        .as_ref()
        .map(|config| static_mode::off(config.mode()))
        .unwrap_or_default();
    let problems = findings
        .iter()
        .filter(|finding| finding.status() == Status::Problem)
        .count();
    let exit = if problems == 0 {
        Exit::Success
    } else {
        Exit::Failure
    };
    match cli.output {
        Output::Json => report::json(
            &DoctorBody {
                site: site.display().to_string(),
                ok: problems == 0,
                problems,
                checks: findings.iter().map(CheckBody::from).collect(),
                off,
            },
            exit,
        ),
        Output::Human => report::text(&human(site, &findings, &off, problems), exit),
    }
}

/// The report for a person: a line per check, each problem as what, why and
/// fix below its check, the features that are `off`, and a last line that
/// says what to do next.
fn human(site: &Path, findings: &[Finding], off: &[static_mode::Off], problems: usize) -> String {
    let mut said = format!("Checked the site in {}.\n", site.display());
    for finding in findings {
        let title = finding.check.title();
        let _ = match &finding.outcome {
            Outcome::Ok(summary) => writeln!(said, "{title}: ok. {summary}"),
            Outcome::Skipped(reason) => writeln!(said, "{title}: skipped. {reason}"),
            Outcome::Problem(message) => writeln!(
                said,
                "{title}: a problem.\n  {}\n  {}\n  {}",
                message.what(),
                message.why(),
                message.fix()
            ),
        };
    }
    said.push_str(&static_mode::human(off));
    let _ = match problems {
        0 => writeln!(said, "No problems found."),
        1 => writeln!(
            said,
            "1 problem to fix, shown above. Run `khalid doctor` again once it is fixed."
        ),
        _ => writeln!(
            said,
            "{problems} problems to fix, shown above. Run `khalid doctor` again once they are fixed."
        ),
    };
    said
}
