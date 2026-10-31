//! `khalid export`: a static site written as plain HTML into `public/`
//! (plan sections 4.1 and 7). It builds into a new folder beside `public/`
//! and swaps it in only once every page is written, so a host that copies
//! `public/` never finds half a site, and pages of deleted posts are gone.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use khalid_content::SiteContent;
use khalid_core::{Clock, Config, Mode, UserMessage};
use khalid_routes::Routes;
use serde::Serialize;
use tempfile::TempDir;

use crate::cli::Output;
use crate::exit::Exit;
use crate::{build, build_message, report};

/// The folder a static site is exported into.
const PUBLIC: &str = "public";

/// The JSON shape of `khalid export --json`.
#[derive(Serialize)]
struct ExportBody {
    out: String,
    pages: usize,
}

/// Exports the static `site`, whose settings, routes and content were
/// checked, into its `public/` folder. `clock` dates a feed with no posts.
pub(crate) fn run(
    site: &Path,
    checked: (&Config, &Routes, &SiteContent),
    clock: &dyn Clock,
    output: Output,
) -> Exit {
    let mode = checked.0.mode();
    if mode != Mode::Static {
        return report::failure(output, &not_static(mode), Exit::Failure);
    }
    let public = within(site, PUBLIC);
    let fresh = match tempfile::Builder::new().prefix(".export-").tempdir_in(site) {
        Ok(fresh) => fresh,
        Err(error) => {
            return report::failure(
                output,
                &build_message::write(&public, &error),
                Exit::Failure,
            );
        }
    };
    // A refused or failed build drops `fresh`, so the earlier export stays.
    let pages = match build::into_folder(site, checked, fresh.path(), clock, output) {
        Ok(pages) => pages,
        Err(exit) => return exit,
    };
    if let Err(error) = swap(site, &public, fresh) {
        return report::failure(
            output,
            &build_message::write(&public, &error),
            Exit::Failure,
        );
    }
    let out = public.display().to_string();
    match output {
        Output::Human => report::text(
            &format!(
                "Exported {pages} pages to {out}, with nothing left from earlier exports.\nCopy that folder to your host.\n"
            ),
            Exit::Success,
        ),
        Output::Json => report::json(&ExportBody { out, pages }, Exit::Success),
    }
}

/// Puts `fresh` where `public` is: the old folder moves aside first and is
/// deleted once the new one is in place, or moved back when that fails.
fn swap(site: &Path, public: &Path, fresh: TempDir) -> io::Result<()> {
    let aside = tempfile::Builder::new()
        .prefix(".export-old-")
        .tempdir_in(site)?;
    let old = within(aside.path(), PUBLIC);
    let had = public.exists();
    if had {
        fs::rename(public, &old)?;
    }
    let fresh = fresh.keep();
    if let Err(error) = fs::rename(&fresh, public) {
        if had {
            drop(fs::rename(&old, public));
        }
        drop(fs::remove_dir_all(&fresh));
        return Err(error);
    }
    Ok(())
}

/// The message when the site is not a static one.
fn not_static(mode: Mode) -> UserMessage {
    let mode = match mode {
        Mode::Headless => "headless",
        _ => "server",
    };
    UserMessage::new(
        format!("This site is in {mode} mode, not static, so there is nothing to export."),
        "An export is the plain HTML a static host serves, and this site is served by `khalid serve`.",
        "Set mode = \"static\" in site.toml to export, or run `khalid build` for a copy of the pages.",
    )
}

/// `name` inside `folder`. Every `name` here is a fixed one.
fn within(folder: &Path, name: &str) -> PathBuf {
    let mut path = folder.to_path_buf();
    path.push(name);
    path
}
