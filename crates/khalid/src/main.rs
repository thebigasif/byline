//! The `khalid` command.
//!
//! This crate owns the edge: it parses the command line, runs a command, and
//! turns every outcome into an exit code and a message a person or an agent can
//! read. It must not know business rules. Those live in the library crates,
//! which this binary calls the same way the API does.

mod assets;
mod atomic;
mod build;
mod build_files;
mod build_gate;
mod build_message;
mod build_pages;
mod build_reader;
mod build_redirects;
mod build_seo;
mod categories;
mod cli;
mod config_message;
mod content_feed;
mod content_message;
mod db;
mod db_message;
mod doctor;
mod doctor_address;
mod doctor_check;
mod doctor_files;
mod doctor_folder;
mod exit;
mod index;
mod init;
mod init_message;
mod init_prompt;
mod keys;
mod keys_message;
mod lint;
mod lint_fix;
mod lint_input;
mod lint_message;
mod new;
mod new_message;
mod outbound;
mod post_files;
mod post_images;
mod post_text;
mod repair;
mod report;
mod routes_message;
mod serve;
mod serve_message;
mod site_content;
mod site_files;
mod site_lists;
mod site_pages;
mod site_posts;
mod slug_repair;
mod theme_check;
mod theme_message;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "khalid theme sync renders the preview, in task 3.8"
    )
)]
mod theme_preview;
mod theme_sync;

use std::ffi::OsString;
use std::path::Path;
use std::process::ExitCode;

use clap::error::ErrorKind;
use clap::{CommandFactory, Parser};
use khalid_content::{ContentTree, SiteContent, check_bodies, read_content};
use khalid_core::{Config, Environment, SystemClock, UserMessage};
use khalid_routes::Routes;
use khalid_server::FolderLook;

use crate::cli::{Cli, Command, Output, ThemeCommand};
use crate::exit::Exit;
use crate::lint::Mode;
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
    if let Some(exit) = run_unchecked(cli) {
        return exit;
    }
    // The server's watch starts from a look taken before the content is
    // read, so a post saved while the server starts still shows.
    let look = match (&cli.command, cli.site.as_deref()) {
        (Some(Command::Serve { .. }), Some(folder)) => Some(FolderLook::take(folder)),
        _ => None,
    };
    let site = match cli.site.as_deref() {
        Some(folder) => match check(folder) {
            Ok((config, routes, content)) => Some((folder, config, routes, content)),
            Err(message) => return report::failure(cli.output, &message, Exit::Failure),
        },
        None => None,
    };
    // The server renders its first pages from the checked content, then
    // drops it, so it takes the content rather than borrowing it.
    let site = match (&cli.command, site) {
        (Some(Command::Serve { listen }), Some((folder, config, routes, content))) => {
            let look = look.unwrap_or_else(|| FolderLook::take(folder));
            let checked = (&config, &routes);
            return serve::run(folder, checked, (content, look), *listen, cli.output);
        }
        (_, site) => site,
    };
    match (&cli.command, &site) {
        (None, _) => report::text(&Cli::command().render_help().to_string(), Exit::Success),
        (Some(Command::Db { command }), Some((folder, ..))) => {
            db::run(folder, *command, cli.output)
        }
        (Some(Command::Lint { path, fix }), Some((folder, ..))) => {
            let mode = if *fix { Mode::Fix } else { Mode::Check };
            lint::run(folder, path.as_deref(), mode, cli.output)
        }
        (Some(Command::New { title, language }), Some((folder, config, ..))) => {
            let post = NewPost {
                title,
                language: language.as_ref(),
            };
            new::run(folder, config, post, &SystemClock, cli.output)
        }
        (Some(Command::Build { out }), Some((folder, config, routes, content))) => build::run(
            folder,
            (config, routes, content),
            out.as_deref(),
            &SystemClock,
            cli.output,
        ),
        (Some(Command::Keys { command }), Some((folder, ..))) => {
            keys::run(folder, command, cli.output)
        }
        (
            Some(Command::Theme {
                command: ThemeCommand::Sync { url },
            }),
            Some((folder, ..)),
        ) => theme_sync::run(folder, url, cli),
        (
            Some(Command::Theme {
                command: ThemeCommand::Check { url },
            }),
            Some((folder, config, ..)),
        ) => theme_check::run(folder, url.as_ref().unwrap_or(config.address()), cli),
        (
            Some(
                command @ (Command::Db { .. }
                | Command::Theme { .. }
                | Command::Serve { .. }
                | Command::Keys { .. }
                | Command::Lint { .. }
                | Command::New { .. }
                | Command::Build { .. }),
            ),
            None,
        ) => report::failure(cli.output, &needs_site(command), Exit::Usage),
        (Some(command), _) => report::failure(cli.output, &not_built(command), Exit::Internal),
    }
}

/// Runs the commands that do not start from a checked site: `init`, which
/// makes the site, and `doctor`, which checks the settings, routes and
/// content itself so it can list every problem instead of stopping at the
/// first. Returns `None` for every other command.
fn run_unchecked(cli: &Cli) -> Option<Exit> {
    match &cli.command {
        Some(Command::Init(args)) => {
            let folder = args
                .folder
                .as_deref()
                .or(cli.site.as_deref())
                .unwrap_or(Path::new("."));
            Some(init::run(folder, args, cli.output))
        }
        Some(command @ Command::Doctor { network }) => Some(match cli.site.as_deref() {
            Some(folder) => doctor::run(folder, *network, cli),
            None => report::failure(cli.output, &needs_site(command), Exit::Usage),
        }),
        _ => None,
    }
}

/// Checks the settings, the routes and the content of the `site` folder, and
/// returns the settings, the routes and the content.
fn check(site: &Path) -> Result<(Config, Routes, SiteContent), UserMessage> {
    let config =
        Config::load(site, &Environment::from_process()).map_err(|e| config_message::of(&e))?;
    let routes = Routes::load(site).map_err(|error| routes_message::of(&error))?;
    let content = ContentTree::walk(site)
        .and_then(|tree| read_content(&tree))
        .and_then(|content| check_bodies(&content, &config, &routes).map(|()| content))
        .map_err(|error| content_message::of(site, &error))?;
    Ok((config, routes, content))
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
