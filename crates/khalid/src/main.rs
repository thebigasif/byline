//! The `khalid` command.
//!
//! This crate owns the edge: it parses the command line, runs a command, and
//! turns every outcome into an exit code and a message a person or an agent can
//! read. It must not know business rules. Those live in the library crates,
//! which this binary calls the same way the API does.

mod agents;
mod announce;
mod assets;
mod atomic;
mod audit_message;
mod backup;
mod backup_message;
mod build;
mod build_files;
mod build_gate;
mod build_home;
mod build_lists;
mod build_message;
mod build_pages;
mod build_reader;
mod build_redirects;
mod build_schema;
mod build_seo;
mod build_sitemap;
mod categories;
mod cli;
mod comment_pages;
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
mod doctor_robots;
mod exit;
mod https_message;
mod import;
mod import_rest;
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
mod listed_post;
mod live_redirects;
mod mcp;
mod mcp_content;
mod mcp_message;
mod mcp_tools;
mod media_library;
mod memory;
mod miss_log;
mod new;
mod new_message;
mod newsletter;
mod outbound;
mod post_files;
mod post_images;
mod post_lock;
mod post_photos;
mod post_text;
mod preview;
mod publish;
mod pull;
mod pull_message;
mod redirect_route;
mod redirects;
mod repair;
mod report;
mod routes_message;
mod scheduler;
mod search_index;
mod search_page;
mod search_route;
mod search_static;
mod seo_crawl_settings;
mod seo_graph_settings;
mod seo_head_settings;
mod seo_settings;
mod serve;
mod serve_https;
mod serve_message;
mod settings_text;
mod share_images;
mod site_addresses;
mod site_audit;
mod site_config;
mod site_content;
mod site_files;
mod site_lists;
mod site_outline;
mod site_page;
mod site_pages;
mod site_posts;
mod slug_moves;
mod slug_repair;
mod staff;
mod staff_message;
mod staff_owner;
mod static_mode;
mod theme_check;
mod theme_message;
mod theme_preview;
mod theme_sync;
mod unlock;
mod update;
mod update_message;
mod update_release;
mod webhooks;

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
use crate::post_files::Assets;

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

/// Readies the process for serving the site in `folder`, before anything
/// is read: small memory pages only, and a look at the folder for the
/// watch.
fn serving(folder: &Path) -> FolderLook {
    memory::small_pages_only();
    FolderLook::take(folder)
}

/// Runs the command the person asked for, after checking the site's settings
/// and reading its content when `--site` names a folder.
fn run(cli: &Cli) -> Exit {
    if let Some(exit) = run_unchecked(cli) {
        return exit;
    }
    let environment = Environment::from_process();
    // The server's watch starts from a look taken before the content is
    // read, so a post saved while the server starts still shows.
    let serves = matches!(cli.command, Some(Command::Serve(_)));
    let look = cli.site.as_deref().filter(|_| serves).map(serving);
    let site = match cli.site.as_deref() {
        Some(folder) => match check(folder, &environment) {
            Ok((config, routes, content, assets)) => {
                Some((folder, config, routes, (content, assets)))
            }
            Err(message) => return report::failure(cli.output, &message, Exit::Failure),
        },
        None => None,
    };
    // The server renders its first pages from the checked content, then
    // drops it, so it takes the content rather than borrowing it.
    let site = match (&cli.command, site) {
        (Some(Command::Serve(args)), Some((folder, config, routes, (content, assets)))) => {
            let look = look.unwrap_or_else(|| FolderLook::take(folder));
            let (checked, start) = ((&config, &routes), (content, assets, look));
            let keys = serve::Keys::read(&environment, &config);
            return serve_https::serve(cli, folder, checked, start, (args, keys));
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
        (
            Some(asked @ (Command::Build { .. } | Command::Audit { .. })),
            Some((folder, config, routes, (content, _))),
        ) => on_content(folder, (config, routes, content), asked, cli),
        (
            Some(command @ (Command::Keys { .. } | Command::Staff { .. })),
            Some((folder, config, ..)),
        ) => access(folder, config, command, cli.output),
        (Some(Command::Mcp), Some((folder, config, ..))) => mcp::run(folder, config, &environment),
        (
            Some(command @ (Command::Theme { .. } | Command::Publish { .. })),
            Some((folder, config, ..)),
        ) => on_settings(folder, config, command, cli),
        (Some(Command::Import(args)), Some((folder, config, routes, _))) => {
            import::run(folder, (config, routes), args, cli)
        }
        (
            Some(
                command @ (Command::Db { .. }
                | Command::Import(_)
                | Command::Theme { .. }
                | Command::Serve(_)
                | Command::Keys { .. }
                | Command::Staff { .. }
                | Command::Mcp
                | Command::Lint { .. }
                | Command::Publish { .. }
                | Command::New { .. }
                | Command::Build { .. }
                | Command::Audit { .. }),
            ),
            None,
        ) => report::failure(cli.output, &needs_site(command), Exit::Usage),
        (Some(command), _) => report::failure(cli.output, &not_built(command), Exit::Internal),
    }
}

/// Runs the commands that need only a checked site's settings: `theme`
/// and `publish`.
fn on_settings(folder: &Path, config: &Config, command: &Command, cli: &Cli) -> Exit {
    match command {
        Command::Theme { command } => theme(folder, config, command, cli),
        Command::Publish { slug, language } => {
            publish::run(cli, (folder, config), (slug, language.as_ref()))
        }
        _ => report::failure(cli.output, &not_built(command), Exit::Internal),
    }
}

/// Runs the commands that read a checked site's content: `build`, and
/// `audit` from a build or the server it names.
fn on_content(
    folder: &Path,
    checked: (&Config, &Routes, &SiteContent),
    asked: &Command,
    cli: &Cli,
) -> Exit {
    match asked {
        Command::Build { out } => {
            build::run(folder, checked, out.as_deref(), &SystemClock, cli.output)
        }
        Command::Audit { server, network } => site_audit::run(
            folder,
            checked,
            (server.as_ref(), *network),
            &SystemClock,
            cli,
        ),
        _ => report::failure(cli.output, &not_built(asked), Exit::Internal),
    }
}

/// Runs `theme sync` and `theme check` on a checked site.
fn theme(folder: &Path, config: &Config, command: &ThemeCommand, cli: &Cli) -> Exit {
    match command {
        ThemeCommand::Sync { url, from_css } => theme_sync::run(
            folder,
            url,
            from_css.as_deref(),
            config.default_language(),
            cli,
        ),
        ThemeCommand::Check { url } => {
            theme_check::run(folder, url.as_ref().unwrap_or(config.address()), cli)
        }
    }
}

/// Runs the commands that decide who may reach the site: `keys` and
/// `staff`.
fn access(folder: &Path, config: &Config, command: &Command, output: Output) -> Exit {
    match command {
        Command::Keys { command } => keys::run(folder, command, output),
        Command::Staff { command } => staff_owner::run(folder, config, command, output),
        _ => report::failure(output, &not_built(command), Exit::Internal),
    }
}

/// Runs the commands that do not start from a checked site: `init`, which
/// makes the site, `agents` and `update`, which need none, `doctor`, which
/// checks the settings, routes and content itself so it can list every
/// problem instead of stopping at the first, and the commands
/// [`on_folder`] runs. Returns `None` for every other command.
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
        Some(Command::Agents) => Some(agents::run(cli.output)),
        Some(Command::Update { target }) => Some(update::run(*target, cli)),
        Some(command @ Command::Doctor { network }) => Some(match cli.site.as_deref() {
            Some(folder) => doctor::run(folder, *network, cli),
            None => report::failure(cli.output, &needs_site(command), Exit::Usage),
        }),
        Some(
            asked @ (Command::Redirects { .. }
            | Command::Webhooks { .. }
            | Command::Backup { .. }
            | Command::Pull),
        ) => Some(match cli.site.as_deref() {
            Some(folder) => on_folder(folder, asked, cli.output),
            None => report::failure(cli.output, &needs_site(asked), Exit::Usage),
        }),
        _ => None,
    }
}

/// Runs the commands that need the site folder but not its checked content:
/// `redirects` and `webhooks`, which read the settings and the database
/// themselves, `backup`, which keeps a broken site as well as a whole one,
/// and `pull`, which checks the site after it brings in the new files.
fn on_folder(folder: &Path, asked: &Command, output: Output) -> Exit {
    match asked {
        Command::Redirects { command } => redirects::run(folder, command, output),
        Command::Webhooks { command } => webhooks::run(folder, command, output),
        Command::Backup { out } => backup::run(folder, out.as_deref(), &SystemClock, output),
        Command::Pull => pull::run(folder, output),
        _ => report::failure(output, &not_built(asked), Exit::Internal),
    }
}

/// What one check of a site folder read: its settings, routes and content,
/// and the files beside its folder posts.
type Checked = (Config, Routes, SiteContent, Assets);

/// Checks the settings, the routes and the content of the `site` folder, and
/// returns them with the files beside folder posts. The folder is walked
/// once, and no file's text is kept beyond the content.
fn check(site: &Path, environment: &Environment) -> Result<Checked, UserMessage> {
    let config = site_config::load(site, environment)?;
    let routes = Routes::load(site).map_err(|error| routes_message::of(&error))?;
    let (content, assets) = ContentTree::walk(site)
        .and_then(|tree| Ok((read_content(&tree)?, Assets::of(&tree))))
        .and_then(|read| check_bodies(&read.0, &config, &routes).map(|()| read))
        .map_err(|error| content_message::of(site, &error))?;
    Ok((config, routes, content, assets))
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
