//! `khalid --site <folder> audit`: build the site into `data/audit/site/`,
//! crawl it the way Ahrefs and Semrush do, and print its health score out
//! of 100 with every issue and its fix. It exits with 1 when it finds an
//! error.

use std::fs;
use std::path::{Path, PathBuf};

use khalid_audit::{Folder, Limit, Report, Severity, Thresholds, crawl};
use khalid_content::SiteContent;
use khalid_core::{AbsoluteUrl, Clock, Config, UserMessage};
use khalid_routes::Routes;

use crate::cli::Output;
use crate::exit::Exit;
use crate::{audit_message, build, report};

/// The folder in `data/` the audit keeps its files in.
const FOLDER: &str = "data/audit";

/// Builds, crawls and audits the checked site in `site`.
pub(crate) fn run(
    site: &Path,
    checked: (&Config, &Routes, &SiteContent),
    clock: &dyn Clock,
    output: Output,
) -> Exit {
    let folder = within(site, FOLDER);
    let built = within(&folder, "site");
    // A page left from an earlier build would be audited as if it were
    // still there, so the folder is made again each time.
    if built.exists()
        && let Err(error) = fs::remove_dir_all(&built)
    {
        return report::failure(output, &audit_message::write(&built, &error), Exit::Failure);
    }
    if let Err(exit) = build::into_folder(site, checked, &built, clock, output) {
        return exit;
    }
    let home = match home(checked.0) {
        Ok(home) => home,
        Err(message) => return report::failure(output, &message, Exit::Internal),
    };
    let source = match Folder::open(&built, &home) {
        Ok(source) => source,
        Err(error) => return report::failure(output, &audit_message::of(&error), Exit::Internal),
    };
    let Ok(runtime) = tokio::runtime::Builder::new_current_thread().build() else {
        return report::failure(output, &audit_message::internal(), Exit::Internal);
    };
    let crawled = runtime.block_on(crawl(&source, &home, Limit::default()));
    let audit = Report::of(&crawled, Thresholds::default());
    let exit = if audit.count(Severity::Error) == 0 {
        Exit::Success
    } else {
        Exit::Failure
    };
    match output {
        Output::Json => report::json(&audit_message::body(&audit), exit),
        Output::Human => report::text(&audit_message::human(&audit), exit),
    }
}

/// The blog's home page: the site's address with its mount.
fn home(config: &Config) -> Result<AbsoluteUrl, UserMessage> {
    let address = config.address().as_str().trim_end_matches('/');
    AbsoluteUrl::parse(&format!("{address}{}", config.mount()))
        .map_err(|_| audit_message::internal())
}

/// `relative` inside `folder`; every `relative` here is a fixed name.
fn within(folder: &Path, relative: &str) -> PathBuf {
    let mut path = folder.to_path_buf();
    path.push(relative);
    path
}
