//! `khalid import`: brings a blog from `WordPress` or Ghost into the site
//! through `khalid-import`, keeping every slug and writing the redirect
//! map beside the posts (plan section 5.7, KH-API-7).

use std::fmt::Display;
use std::fs;
use std::path::{Path, PathBuf};

use khalid_core::{Config, UserMessage};
use khalid_import::{WriteError, ghost, wordpress};
use khalid_routes::Routes;
use serde::Serialize;

use crate::cli::{ImportArgs, ImportSource, Output, Run};
use crate::exit::Exit;
use crate::report;

/// The JSON shape of `khalid import --json`.
#[derive(Serialize)]
struct ImportedBody<'a> {
    dry_run: bool,
    report: &'a str,
    written: Vec<String>,
}

/// One source's plan, which says what an import would write.
enum Plan {
    Wordpress(wordpress::Plan),
    Ghost(ghost::Plan),
}

impl Plan {
    /// The dry-run report, in plain sentences.
    fn report(&self) -> String {
        match self {
            Self::Wordpress(plan) => plan.report().to_string(),
            Self::Ghost(plan) => plan.report().to_string(),
        }
    }

    fn write(&self, site: &Path) -> Result<Vec<PathBuf>, WriteError> {
        match self {
            Self::Wordpress(plan) => plan.write(site),
            Self::Ghost(plan) => plan.write(site),
        }
    }

    /// The redirect map's file, from the site folder.
    fn map_file(&self) -> &'static str {
        match self {
            Self::Wordpress(_) => wordpress::REDIRECTS_CSV,
            Self::Ghost(_) => ghost::REDIRECTS_CSV,
        }
    }
}

/// Runs `khalid import` on the checked site in `site`: a dry run prints the
/// report, and a real run writes the files.
pub(crate) fn run(
    site: &Path,
    checked: (&Config, &Routes),
    args: &ImportArgs,
    output: Output,
) -> Exit {
    let (config, routes) = checked;
    planned(args, config, routes)
        .and_then(|plan| match args.run {
            Run::Dry => Ok(answer(&plan.report(), None, output)),
            Run::Write => {
                let written = plan
                    .write(site)
                    .map_err(|error| (not_written(&error), Exit::Failure))?;
                // The report's first sentence is for a dry run only.
                let said = plan.report();
                let said = said.split_once('\n').map_or("", |(_, rest)| rest);
                Ok(answer(said, Some((&written, plan.map_file())), output))
            }
        })
        .unwrap_or_else(|(message, exit)| report::failure(output, &message, exit))
}

/// Prints what would come in, or with `done`, the files written and the
/// redirect map's file.
fn answer(said: &str, done: Option<(&[PathBuf], &str)>, output: Output) -> Exit {
    match (output, done) {
        (Output::Json, _) => {
            let (written, _) = done.unwrap_or_default();
            let body = ImportedBody {
                dry_run: done.is_none(),
                report: said,
                written: written.iter().map(|p| p.display().to_string()).collect(),
            };
            report::json(&body, Exit::Success)
        }
        (Output::Human, None) => report::text(said, Exit::Success),
        (Output::Human, Some((written, map))) => {
            let files = written.len();
            let text = format!(
                "Wrote {files} files.\n{said}\
                 Run `khalid redirects import {map}` to store the redirects.\n"
            );
            report::text(&text, Exit::Success)
        }
    }
}

/// Reads the export the person named and plans the import.
fn planned(
    args: &ImportArgs,
    config: &Config,
    routes: &Routes,
) -> Result<Plan, (UserMessage, Exit)> {
    let export = args.export.as_path();
    match args.source {
        ImportSource::Wordpress => {
            let wxr = wordpress::read_wxr(&read(export)?).map_err(|e| unreadable(export, &e))?;
            let rules = match &args.redirection {
                Some(file) => Some(
                    wordpress::read_redirection(&read(file)?).map_err(|e| unreadable(file, &e))?,
                ),
                None => None,
            };
            Ok(Plan::Wordpress(wordpress::plan(
                &wxr,
                rules.as_ref(),
                config,
                routes,
            )))
        }
        ImportSource::Ghost => {
            let read = ghost::read_export(&read(export)?).map_err(|e| unreadable(export, &e))?;
            let old_site = args.old_site.as_ref().unwrap_or(config.address());
            Ok(Plan::Ghost(ghost::plan(&read, old_site, config, routes)))
        }
        ImportSource::Medium | ImportSource::Substack | ImportSource::Markdown => Err((
            UserMessage::new(
                "That source cannot be imported yet.",
                "Khalid reads WordPress and Ghost exports so far.",
                "Run `khalid import --help` to see the sources that work.",
            ),
            Exit::Internal,
        )),
    }
}

fn read(file: &Path) -> Result<String, (UserMessage, Exit)> {
    fs::read_to_string(file).map_err(|error| not_read(file, &error))
}

fn not_read(file: &Path, error: &impl Display) -> (UserMessage, Exit) {
    let message = UserMessage::new(
        format!("Khalid could not read {}: {error}.", file.display()),
        "Nothing was imported.",
        "Check the path, and unpack a zip archive first, then run the command again.",
    );
    (message, Exit::Usage)
}

fn unreadable(file: &Path, error: &impl Display) -> (UserMessage, Exit) {
    let message = UserMessage::new(
        format!("Khalid cannot import {}: {error}.", file.display()),
        "Nothing was imported, so the site holds no half blog.",
        "Export the blog again from its platform, then run the command again.",
    );
    (message, Exit::Failure)
}

fn not_written(error: &WriteError) -> UserMessage {
    let fix = match error {
        WriteError::Exists { paths } => format!(
            "Move these files away, or import into a new site: {}.",
            paths
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        _ => "Check that the site folder can be written to, then run the command again.".to_owned(),
    };
    UserMessage::new(
        format!("The import could not write its files: {error}."),
        "Readers still reach the old blog, since nothing points to the new one yet.",
        fix,
    )
}
