//! The `khalid` command.
//!
//! This crate owns the edge: it parses the command line, runs a command, and
//! turns every outcome into an exit code and a message a person or an agent can
//! read. It must not know business rules. Those live in the library crates,
//! which this binary calls the same way the API does.

mod cli;
mod config_message;
mod content_message;
mod exit;
mod report;
mod routes_message;

use std::ffi::OsString;
use std::process::ExitCode;

use clap::error::ErrorKind;
use clap::{CommandFactory, Parser};
use khalid_content::{ContentTree, read_content};
use khalid_core::{Config, Environment, UserMessage};
use khalid_routes::Routes;

use crate::cli::{Cli, Command, Output};
use crate::exit::Exit;

#[global_allocator]
static ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().collect();
    let exit = match Cli::try_parse_from(&args) {
        Ok(cli) => run(&cli),
        Err(error) => parse_failure(&error, Output::requested_in(&args)),
    };
    exit.into()
}

/// Runs the command the person asked for, after checking the site's settings
/// and reading its content when `--site` names a folder.
fn run(cli: &Cli) -> Exit {
    if let Some(site) = &cli.site {
        if let Err(error) = Config::load(site, &Environment::from_process()) {
            return report::failure(cli.output, &config_message::of(&error), Exit::Failure);
        }
        if let Err(error) = Routes::load(site) {
            return report::failure(cli.output, &routes_message::of(&error), Exit::Failure);
        }
        if let Err(error) = ContentTree::walk(site).and_then(|tree| read_content(&tree)) {
            return report::failure(
                cli.output,
                &content_message::of(site, &error),
                Exit::Failure,
            );
        }
    }
    match &cli.command {
        None => report::text(&Cli::command().render_help().to_string(), Exit::Success),
        Some(command) => report::failure(cli.output, &not_built(command), Exit::Internal),
    }
}

/// The message for a command that a later phase builds.
fn not_built(command: &Command) -> UserMessage {
    UserMessage::new(
        format!("The command `khalid {}` is not built yet.", command.name()),
        "Khalid is in its first phase, and this command arrives in a later one.",
        "Run `khalid --help` to see every command, and check the next release.",
    )
}

/// Answers `--help`, `--version` and command lines `clap` could not read.
fn parse_failure(error: &clap::Error, output: Output) -> Exit {
    match error.kind() {
        ErrorKind::DisplayHelp => report::text(&error.render().to_string(), Exit::Success),
        ErrorKind::DisplayVersion => report::version(output),
        _ => report::failure(output, &usage_message(error), Exit::Usage),
    }
}

/// Turns a `clap` error into what, why and fix.
fn usage_message(error: &clap::Error) -> UserMessage {
    let rendered = error.render().to_string();
    let detail: Vec<&str> = rendered
        .lines()
        .map(str::trim)
        .take_while(|line| !line.starts_with("Usage:"))
        .filter(|line| !line.is_empty())
        .collect();
    let detail = detail.join(" ");
    let detail = detail.strip_prefix("error: ").unwrap_or(&detail);
    UserMessage::new(
        format!(
            "Khalid could not read the command line: {}.",
            detail.trim_end_matches('.')
        ),
        "Khalid stopped before doing anything, so nothing has changed.",
        "Run `khalid --help` to see the commands and their options.",
    )
}

impl Output {
    /// Whether `--json` appears in arguments that `clap` could not parse.
    fn requested_in(args: &[OsString]) -> Self {
        if args.iter().any(|arg| arg == "--json") {
            Self::Json
        } else {
            Self::Human
        }
    }
}
