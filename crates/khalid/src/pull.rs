//! `khalid pull`: in git mode, the site's content repository brought up to
//! what was pushed to it, then checked like any other start (plan sections
//! 4.1 and 5.14). It only moves forward: changes made in the blog's own
//! copy are never merged or overwritten. A running `khalid serve` sees the
//! new files through its watch.

use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output as Ran, Stdio};

use khalid_core::{Environment, UserMessage};
use serde::Serialize;

use crate::cli::Output;
use crate::exit::Exit;
use crate::{pull_message, report};

/// The JSON shape of `khalid pull --json`.
#[derive(Serialize)]
struct PullBody {
    from: String,
    to: String,
    changed: bool,
    files: usize,
}

/// Where the pull moved the repository: the commits before and after, and
/// how many files changed between them.
struct Moved {
    from: String,
    to: String,
    files: usize,
}

/// Pulls the repository holding `site`, then checks the site.
pub(crate) fn run(site: &Path, output: Output) -> Exit {
    let moved = match pulled(site) {
        Ok(moved) => moved,
        Err(message) => return report::failure(output, &message, Exit::Failure),
    };
    if let Err(message) = crate::check(site, &Environment::from_process()) {
        return report::failure(output, &message, Exit::Failure);
    }
    success(output, &moved)
}

/// Fast-forwards the repository holding `site` to its upstream branch, and
/// says where it moved.
fn pulled(site: &Path) -> Result<Moved, UserMessage> {
    if !within(site, "site.toml").is_file() {
        return Err(pull_message::not_a_site(site));
    }
    let head = |site| git(site, &["rev-parse", "HEAD"]).map(|ran| line(&ran.stdout));
    let inside = git(site, &["rev-parse", "--is-inside-work-tree"])?;
    if !inside.status.success() {
        return Err(pull_message::not_a_repository(site));
    }
    let from = head(site)?;
    let ran = git(site, &["pull", "--ff-only", "--quiet"])?;
    if !ran.status.success() {
        return Err(pull_message::refused(site, &line(&ran.stderr)));
    }
    let to = head(site)?;
    let files = if from == to {
        0
    } else {
        let range = format!("{from}..{to}");
        let ran = git(site, &["diff", "--name-only", &range])?;
        String::from_utf8_lossy(&ran.stdout).lines().count()
    };
    Ok(Moved { from, to, files })
}

/// Runs git in `site` with `args`, never asking for a password on the
/// terminal, so a pull from a script fails instead of waiting.
fn git(site: &Path, args: &[&str]) -> Result<Ran, UserMessage> {
    Command::new("git")
        .arg("-C")
        .arg(site)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .output()
        .map_err(|error: io::Error| match error.kind() {
            io::ErrorKind::NotFound => pull_message::no_git(),
            _ => pull_message::refused(site, &error.to_string()),
        })
}

/// The first line of what git printed.
fn line(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .to_owned()
}

/// Says what the pull brought in, and what happens to it next.
fn success(output: Output, moved: &Moved) -> Exit {
    let changed = moved.files > 0;
    if output == Output::Json {
        let body = PullBody {
            from: moved.from.clone(),
            to: moved.to.clone(),
            changed,
            files: moved.files,
        };
        return report::json(&body, Exit::Success);
    }
    let short = |commit: &str| commit.chars().take(7).collect::<String>();
    let next = "A running `khalid serve` shows the changes on its own.";
    let said = match moved.files {
        0 => "The content repository has nothing new, so nothing changed.".to_owned(),
        1 => format!(
            "Pulled 1 changed file, from {} to {}.\n{next}",
            short(&moved.from),
            short(&moved.to)
        ),
        files => format!(
            "Pulled {files} changed files, from {} to {}.\n{next}",
            short(&moved.from),
            short(&moved.to)
        ),
    };
    report::text(&format!("{said}\n"), Exit::Success)
}

/// `name` inside `folder`. Every `name` here is a fixed one.
fn within(folder: &Path, name: &str) -> PathBuf {
    let mut path = folder.to_path_buf();
    path.push(name);
    path
}
