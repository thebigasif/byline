//! The `khalid` command.
//!
//! This crate owns the edge: it parses the command line, runs a command, and
//! turns every outcome into an exit code and a message a person or an agent can
//! read. It must not know business rules. Those live in the library crates,
//! which this binary calls the same way the API does.

mod atomic;
mod build;
mod build_message;
mod build_pages;
mod cli;
mod config_message;
mod content_message;
mod db;
mod db_message;
mod exit;
mod index;
mod init;
mod init_message;
mod init_prompt;
mod new;
mod new_message;
mod report;
mod routes_message;

use std::ffi::OsString;
use std::path::Path;
use std::process::ExitCode;

use clap::error::ErrorKind;
use clap::{CommandFactory, Parser};
use khalid_content::{ContentTree, read_content};
use khalid_core::{Config, Environment, SystemClock, UserMessage};
use khalid_routes::Routes;

use crate::cli::{Cli, Command, Output};
use crate::exit::Exit;
use crate::new::NewPost;

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
    if let Some(Command::Init(args)) = &cli.command {
        let folder = args
            .folder
            .as_deref()
            .or(cli.site.as_deref())
            .unwrap_or(Path::new("."));
        return init::run(folder, args, cli.output);
    }
    let site = match cli.site.as_deref() {
        Some(folder) => match check(folder) {
            Ok(config) => Some((folder, config)),
            Err(message) => return report::failure(cli.output, &message, Exit::Failure),
        },
        None => None,
    };
    match (&cli.command, &site) {
        (None, _) => report::text(&Cli::command().render_help().to_string(), Exit::Success),
        (Some(Command::Db { command }), Some((folder, _))) => db::run(folder, *command, cli.output),
        (Some(Command::New { title, language }), Some((folder, config))) => {
            let post = NewPost {
                title,
                language: language.as_ref(),
            };
            new::run(folder, config, post, &SystemClock, cli.output)
        }
        (Some(Command::Build { out }), Some((folder, config))) => {
            build::run(folder, config, out.as_deref(), cli.output)
        }
        (
            Some(command @ (Command::Db { .. } | Command::New { .. } | Command::Build { .. })),
            None,
        ) => report::failure(cli.output, &needs_site(command), Exit::Usage),
        (Some(command), _) => report::failure(cli.output, &not_built(command), Exit::Internal),
    }
}

/// Checks the settings, the routes and the content of the `site` folder, and
/// returns the settings.
fn check(site: &Path) -> Result<Config, UserMessage> {
    let config =
        Config::load(site, &Environment::from_process()).map_err(|e| config_message::of(&e))?;
    Routes::load(site).map_err(|error| routes_message::of(&error))?;
    ContentTree::walk(site)
        .and_then(|tree| read_content(&tree))
        .map_err(|error| content_message::of(site, &error))?;
    Ok(config)
}

/// The message for a command that works on one site folder when none was given.
fn needs_site(command: &Command) -> UserMessage {
    UserMessage::new(
        format!(
            "The command `khalid {}` needs a site folder.",
            command.name()
        ),
        "It works on the files inside one site, so Khalid has to know which one.",
        format!(
            "Run `khalid --site <folder> {}` with the folder that holds site.toml.",
            command.name()
        ),
    )
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
        .take_while(|line| !line.starts_with("Usage:") && !line.starts_with("For more information"))
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
