//! The `byline` command.
//!
//! This crate owns the edge: it parses the command line, runs a command, and
//! turns every outcome into an exit code and a message a person or an agent can
//! read. It must not know business rules. Those live in the library crates,
//! which this binary calls the same way the API does.

mod cli;
mod exit;
mod report;

use std::process::ExitCode;

use byline_core::UserMessage;
use clap::{CommandFactory, Parser};

use crate::cli::{Cli, Command};
use crate::exit::Exit;

#[global_allocator]
static ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() -> ExitCode {
    // `parse` answers `--help` and `--version`, and refuses a command line it
    // cannot read with exit code 2.
    run(&Cli::parse()).into()
}

/// Runs the command the person asked for.
fn run(cli: &Cli) -> Exit {
    match &cli.command {
        None => report::text(&Cli::command().render_help().to_string(), Exit::Success),
        Some(command) => report::failure(&not_built(command), Exit::Internal),
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
