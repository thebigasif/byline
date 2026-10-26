//! The `byline` command.
//!
//! This crate owns the edge: it parses the command line, runs a command, and
//! turns every outcome into an exit code and a message a person or an agent can
//! read. It must not know business rules. Those live in the library crates,
//! which this binary calls the same way the API does.

mod cli;
mod exit;
mod report;

use std::ffi::OsString;
use std::process::ExitCode;

use byline_core::UserMessage;
use clap::error::ErrorKind;
use clap::{CommandFactory, Parser};

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

/// Runs the command the person asked for.
fn run(cli: &Cli) -> Exit {
    match &cli.command {
        None => report::text(&Cli::command().render_help().to_string(), Exit::Success),
        Some(command) => report::failure(cli.output, &not_built(command), Exit::Internal),
    }
}

/// The message for a command that a later phase builds.
fn not_built(command: &Command) -> UserMessage {
    UserMessage::new(
        format!("The command `byline {}` is not built yet.", command.name()),
        "Byline is in its first phase, and this command arrives in a later one.",
        "Run `byline --help` to see every command, and check the next release.",
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
            "Byline could not read the command line: {}.",
            detail.trim_end_matches('.')
        ),
        "Byline stopped before doing anything, so nothing has changed.",
        "Run `byline --help` to see the commands and their options.",
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

/// 40 MB of data to prove the binary size budget fails in CI. Never merged.
static BLOAT: [u8; 40_000_000] = [7; 40_000_000];

#[used]
static KEEP: &[u8] = &BLOAT;
