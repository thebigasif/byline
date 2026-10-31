//! `khalid new --branch`: git mode, where each draft is a branch named
//! after the post and publishing is the merge (plan section 4.6). Khalid
//! runs the `git` program, as `khalid pull` does, with the writer's own
//! settings, and never pushes.

use std::io;
use std::path::Path;
use std::process::{Command, Output as Ran, Stdio};

use khalid_core::{Slug, UserMessage};

use crate::new_message;

/// The branch a draft of the post with `slug` lives on.
pub(crate) fn branch_for(slug: &Slug) -> String {
    format!("post/{slug}")
}

/// Starts `branch` from where the repository holding `site` stands, and
/// switches to it, once it is sure the branch is new.
///
/// # Errors
///
/// A message when git cannot run, `site` is not in a repository, or the
/// branch is there already.
pub(crate) fn start(site: &Path, branch: &str) -> Result<(), UserMessage> {
    if !git(site, &["rev-parse", "--is-inside-work-tree"])?
        .status
        .success()
    {
        return Err(new_message::not_a_repository(site));
    }
    let reference = format!("refs/heads/{branch}");
    let known = git(site, &["rev-parse", "--verify", "--quiet", &reference])?;
    if known.status.success() {
        return Err(new_message::branch_exists(branch));
    }
    let switched = git(site, &["switch", "--quiet", "--create", branch])?;
    if !switched.status.success() {
        return Err(new_message::git_refused(site, &line(&switched.stderr)));
    }
    Ok(())
}

/// Commits the post at `relative` in `site`, titled `title`, and only it.
///
/// # Errors
///
/// A message with git's first line when it refuses, such as when the
/// writer has no name set for commits.
pub(crate) fn commit(site: &Path, relative: &str, title: &str) -> Result<(), UserMessage> {
    let subject = format!("Draft: {title}");
    let steps: [&[&str]; 2] = [
        &["add", "--", relative],
        &["commit", "--quiet", "-m", &subject, "--", relative],
    ];
    for step in steps {
        let ran = git(site, step)?;
        if !ran.status.success() {
            return Err(new_message::git_refused(site, &line(&ran.stderr)));
        }
    }
    Ok(())
}

/// Runs git in `site` with `args`, never asking anything on the terminal.
fn git(site: &Path, args: &[&str]) -> Result<Ran, UserMessage> {
    Command::new("git")
        .arg("-C")
        .arg(site)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .output()
        .map_err(|error: io::Error| match error.kind() {
            io::ErrorKind::NotFound => new_message::no_git(),
            _ => new_message::git_refused(site, &error.to_string()),
        })
}

/// The first line of what git printed.
fn line(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    text.lines().next().unwrap_or_default().trim().to_owned()
}
