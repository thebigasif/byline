//! `khalid import`: brings a blog from `WordPress`, Ghost, Medium, Substack or a markdown folder into the
//! site through `khalid-import`, keeping every slug, and stores the
//! redirect map in `data/site.db` through the redirect desk (plan section
//! 5.7, KH-API-7, KH-SEO-8).

use std::fmt::Display;
use std::fs;
use std::path::{Path, PathBuf};

use khalid_api::redirect_store::Origin;
use khalid_core::{AbsoluteUrl, Config, UserMessage};
use khalid_import::{PlannedFile, WriteError, folder, ghost, medium, substack, wordpress};
use khalid_routes::Routes;
use khalid_seo::read_redirects_csv;
use serde::Serialize;

use crate::cli::{Cli, ImportArgs, ImportSource, Output, Run};
use crate::exit::Exit;
use crate::{import_rest, redirects, report};

/// The JSON shape of `khalid import --json`.
#[derive(Serialize)]
struct ImportedBody<'a> {
    dry_run: bool,
    report: &'a str,
    written: Vec<String>,
    stored_redirects: usize,
}

/// One source's plan, which says what an import would write.
enum Plan {
    Wordpress(wordpress::Plan),
    Ghost(ghost::Plan),
    Medium(medium::Plan),
    Substack(substack::Plan),
    Folder(folder::Plan),
}

impl Plan {
    /// The dry-run report, in plain sentences.
    fn report(&self) -> String {
        match self {
            Self::Wordpress(plan) => plan.report().to_string(),
            Self::Ghost(plan) => plan.report().to_string(),
            Self::Medium(plan) => plan.report().to_string(),
            Self::Substack(plan) => plan.report().to_string(),
            Self::Folder(plan) => plan.report().to_string(),
        }
    }

    fn files(&self) -> &[PlannedFile] {
        match self {
            Self::Wordpress(plan) => &plan.files,
            Self::Ghost(plan) => &plan.files,
            Self::Medium(plan) => &plan.files,
            Self::Substack(plan) => &plan.files,
            Self::Folder(plan) => &plan.files,
        }
    }

    fn write(&self, site: &Path) -> Result<Vec<PathBuf>, WriteError> {
        match self {
            Self::Wordpress(plan) => plan.write(site),
            Self::Ghost(plan) => plan.write(site),
            Self::Medium(plan) => plan.write(site),
            Self::Substack(plan) => plan.write(site),
            Self::Folder(plan) => plan.write(site),
        }
    }

    /// The redirect map's file, from the site folder.
    fn map_file(&self) -> &'static str {
        match self {
            Self::Wordpress(_) => wordpress::REDIRECTS_CSV,
            Self::Ghost(_) => ghost::REDIRECTS_CSV,
            Self::Medium(_) => medium::REDIRECTS_CSV,
            Self::Substack(_) => substack::REDIRECTS_CSV,
            Self::Folder(_) => folder::REDIRECTS_CSV,
        }
    }
}

type Done = Result<Exit, (UserMessage, Exit)>;

/// Runs `khalid import` on the checked site in `site`: a dry run prints the
/// report, and a real run writes the files and stores the redirects.
pub(crate) fn run(site: &Path, checked: (&Config, &Routes), args: &ImportArgs, cli: &Cli) -> Exit {
    let (config, routes, output) = (checked.0, checked.1, cli.output);
    planned(args, (config, routes), cli)
        .and_then(|plan| match args.run {
            Run::Dry => Ok(answer(&plan.report(), None, output)),
            Run::Write => import(site, &plan, output),
        })
        .unwrap_or_else(|(message, exit)| report::failure(output, &message, exit))
}

/// Writes the plan's files, then stores the redirects that start on this
/// site's host.
fn import(site: &Path, plan: &Plan, output: Output) -> Done {
    let written = plan
        .write(site)
        .map_err(|error| (not_written(&error), Exit::Failure))?;
    let map = plan.map_file();
    let csv = plan
        .files()
        .iter()
        .find(|file| file.path == Path::new(map))
        .map(|file| ours(&file.text))
        .unwrap_or_default();
    let stored = store(site, &csv).map_err(|(why, exit)| {
        let message = UserMessage::new(
            format!("The posts came in, but the redirects in {map} were not stored."),
            why,
            format!("Fix that, then run `khalid redirects import {map}`."),
        );
        (message, exit)
    })?;
    // The report's first sentence is for a dry run only.
    let said = plan.report();
    let said = said.split_once('\n').map_or("", |(_, rest)| rest);
    Ok(answer(said, Some((&written, stored)), output))
}

/// Stores every redirect in `csv` through the redirect desk, and returns
/// how many, or says why not in one sentence.
fn store(site: &Path, csv: &str) -> Result<usize, (String, Exit)> {
    let entries = read_redirects_csv(csv)
        .map_err(|error| (format!("The map cannot be read: {error}."), Exit::Internal))?;
    if entries.is_empty() {
        return Ok(0);
    }
    redirects::on_desk(site, async |desk| {
        desk.store_checked(entries, Origin::Import).await
    })
    .map_err(|(message, exit)| (message.what().to_owned(), exit))?
    .map(|rows| rows.len())
    .map_err(|refusal| {
        (
            format!("They clash with the stored ones: {refusal}."),
            Exit::Failure,
        )
    })
}

/// The rows of a redirect map that this site can answer: those whose old
/// address is a path or a pattern. A row holding a whole address on another
/// host stays in the file for that host.
fn ours(csv: &str) -> String {
    let mut kept = String::new();
    for (index, line) in csv.lines().enumerate() {
        let path = line.trim_start_matches('"').starts_with('/');
        if index == 0 || path || line.ends_with(",regex") {
            kept.push_str(line);
            kept.push('\n');
        }
    }
    kept
}

/// Prints what would come in, or with `done`, the files written and the
/// redirects stored.
fn answer(said: &str, done: Option<(&[PathBuf], usize)>, output: Output) -> Exit {
    match (output, done) {
        (Output::Json, _) => {
            let (written, stored) = done.unwrap_or_default();
            let body = ImportedBody {
                dry_run: done.is_none(),
                report: said,
                written: written.iter().map(|p| p.display().to_string()).collect(),
                stored_redirects: stored,
            };
            report::json(&body, Exit::Success)
        }
        (Output::Human, None) => report::text(said, Exit::Success),
        (Output::Human, Some((written, stored))) => {
            let files = written.len();
            let text = format!(
                "Wrote {files} files and stored {stored} redirects.\n{said}\
                 Run `khalid lint` to see how each post scores.\n"
            );
            report::text(&text, Exit::Success)
        }
    }
}

/// Reads the export the person named and plans the import.
fn planned(
    args: &ImportArgs,
    (config, routes): (&Config, &Routes),
    cli: &Cli,
) -> Result<Plan, (UserMessage, Exit)> {
    let export = args.export.as_path();
    match args.source {
        ImportSource::Wordpress => {
            let named = export.to_string_lossy();
            let wxr = if import_rest::is_address(&named) {
                let site = AbsoluteUrl::parse(&named).map_err(|e| unreadable(export, &e))?;
                let answers = import_rest::answers(cli, &site)?;
                wordpress::read_rest(&answers).map_err(|e| unreadable(export, &e))?
            } else {
                wordpress::read_wxr(&read(export)?).map_err(|e| unreadable(export, &e))?
            };
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
        ImportSource::Medium => {
            let files = posts_folder(export)?;
            let read = medium::read_export(files).map_err(|e| unreadable(export, &e))?;
            Ok(Plan::Medium(medium::plan(&read, config, routes)))
        }
        ImportSource::Substack => {
            let mut list = export.to_path_buf();
            list.push("posts.csv");
            let read = substack::read_export(&read(&list)?, posts_folder(export)?)
                .map_err(|e| unreadable(export, &e))?;
            let old_site = args.old_site.as_ref().unwrap_or(config.address());
            Ok(Plan::Substack(substack::plan(
                &read, old_site, config, routes,
            )))
        }
        ImportSource::Markdown => {
            let mut files = Vec::new();
            markdown_files(export, "", &mut files)?;
            let read = folder::read_folder(files).map_err(|e| unreadable(export, &e))?;
            let old_site = args.old_site.as_ref().unwrap_or(config.address());
            Ok(Plan::Folder(folder::plan(&read, old_site, config, routes)))
        }
    }
}

/// Folders a generator builds into or fills with packages, which hold no
/// posts of their own.
const BUILT: [&str; 4] = ["node_modules", "public", "_site", "resources"];

/// Adds every markdown file under `folder`, with its path from the top
/// starting `inside`, to `files`. Hidden and built folders are skipped.
fn markdown_files(
    folder: &Path,
    inside: &str,
    files: &mut Vec<(String, String)>,
) -> Result<(), (UserMessage, Exit)> {
    let entries = fs::read_dir(folder).map_err(|error| not_read(folder, &error))?;
    for entry in entries {
        let path = entry.map_err(|error| not_read(folder, &error))?.path();
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned());
        let name = name.unwrap_or_default();
        let at = format!("{inside}{name}");
        if path.is_dir() && !name.starts_with('.') && !BUILT.contains(&name.as_str()) {
            markdown_files(&path, &format!("{at}/"), files)?;
        } else if [".md", ".markdown", ".mdx"]
            .iter()
            .any(|ext| name.ends_with(ext))
        {
            let text = fs::read(&path).map_err(|error| not_read(&path, &error))?;
            files.push((at, String::from_utf8_lossy(&text).into_owned()));
        }
    }
    Ok(())
}

/// Every file in a Medium or Substack export's `posts/` folder, by name.
/// `folder` may be the unpacked export or its `posts/` folder.
fn posts_folder(folder: &Path) -> Result<Vec<(String, String)>, (UserMessage, Exit)> {
    let mut posts = folder.to_path_buf();
    posts.push("posts");
    let posts = if posts.is_dir() {
        posts
    } else {
        folder.to_path_buf()
    };
    let entries = fs::read_dir(&posts).map_err(|error| not_read(&posts, &error))?;
    let mut files = Vec::new();
    for entry in entries {
        let path = entry.map_err(|error| not_read(&posts, &error))?.path();
        if path.is_file() {
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned());
            let text = fs::read(&path).map_err(|error| not_read(&path, &error))?;
            files.push((
                name.unwrap_or_default(),
                String::from_utf8_lossy(&text).into_owned(),
            ));
        }
    }
    Ok(files)
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
        "No redirect was stored, so readers still reach the old addresses.",
        fix,
    )
}
