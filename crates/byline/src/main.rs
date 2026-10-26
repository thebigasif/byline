//! The `byline` command.
//!
//! This crate owns the edge: it parses the command line, runs a command, and
//! turns every outcome into an exit code and a message a person or an agent can
//! read. It must not know business rules. Those live in the library crates,
//! which this binary calls the same way the API does.

use std::io::{self, Write};
use std::process::ExitCode;

use clap::{CommandFactory, Parser};

#[global_allocator]
static ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// The command line.
#[derive(Debug, Parser)]
#[command(
    name = "byline",
    version,
    about = "An open-source blog engine with SEO defaults built in, by SerpDad.",
    long_about = None
)]
struct Cli {}

fn main() -> ExitCode {
    // `parse` answers `--help` and `--version`, and refuses anything else it
    // does not know with exit code 2.
    let _cli = Cli::parse();
    let help = Cli::command().render_help().to_string();
    match write!(io::stdout().lock(), "{help}") {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::from(3),
    }
}
