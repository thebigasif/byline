//! `khalid --site <folder> audit`: build the site into `data/audit/site/`,
//! or ask a running server with `--server`, crawl it the way Ahrefs and
//! Semrush do, and print its health score out of 100 with every issue and
//! its fix. It keeps each run's score in
//! `data/audit/runs.jsonl` for the trend, and writes the issues, the
//! internal linking report and the page-weight meter as CSV beside it. It
//! names the heaviest page. It exits with 1 when it finds
//! an error. The AI search checks read the answer length, the paragraph
//! length and the stale window from the site's `lint.toml`, like the
//! linter.

use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};

use khalid_audit::{
    Folder, Limit, LinkReport, Report, Run, Server, Severity, Thresholds, WeightReport,
    check_outside, crawl, history,
};
use khalid_content::SiteContent;
use khalid_core::{AbsoluteUrl, Clock, Config, UserMessage};
use khalid_outbound::{Limits, Outbound, Resolver};
use khalid_routes::Routes;

use crate::cli::{Cli, Network, Output};
use crate::exit::Exit;
use crate::{audit_message, build, lint, outbound, report};

/// The folder in `data/` the audit keeps its files in.
const FOLDER: &str = "data/audit";

/// Audits the checked site in `site`: the running `server` when one is
/// given, or else the site built into `data/audit/site/`. Unless `network`
/// says to stay offline, it then asks every address on another site the
/// pages link to or load, through the public client.
pub(crate) fn run(
    site: &Path,
    checked: (&Config, &Routes, &SiteContent),
    (server, network): (Option<&AbsoluteUrl>, Network),
    clock: &dyn Clock,
    cli: &Cli,
) -> Exit {
    let output = cli.output;
    let folder = within(site, FOLDER);
    let config = checked.0;
    let thresholds = match thresholds(site, clock) {
        Ok(thresholds) => thresholds,
        Err(message) => return report::failure(output, &message, Exit::Failure),
    };
    let (Ok(home), Ok(runtime)) = (
        home(config),
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build(),
    ) else {
        return report::failure(output, &audit_message::internal(), Exit::Internal);
    };
    let crawled = if let Some(server) = server {
        let limits = Limits::default().with_redirects(0);
        let Ok(outbound) = Outbound::with_resolver(limits, Resolver::for_site_under_audit()) else {
            return report::failure(output, &audit_message::internal(), Exit::Internal);
        };
        let source = Server::new(outbound, config.address(), server);
        runtime.block_on(crawl(&source, &home, Limit::default()))
    } else {
        let source = match built(site, checked, &folder, clock, output) {
            Ok(built) => built,
            Err(exit) => return exit,
        };
        runtime.block_on(crawl(&source, &home, Limit::default()))
    };
    let mut audit = Report::of(&crawled, thresholds);
    if network == Network::Fetch {
        let Ok(public) = outbound::client(cli) else {
            return report::failure(output, &audit_message::internal(), Exit::Internal);
        };
        audit = audit.with(runtime.block_on(check_outside(&crawled, &public)));
    }
    let links = LinkReport::of(&crawled);
    let weights = WeightReport::of(&crawled);
    let trend = match keep(&folder, &audit, (&links, &weights), clock) {
        Ok(trend) => trend,
        Err(message) => return report::failure(output, &message, Exit::Failure),
    };
    let exit = if audit.count(Severity::Error) == 0 {
        Exit::Success
    } else {
        Exit::Failure
    };
    match output {
        Output::Json => {
            let body = audit_message::body((&audit, &weights), &trend, &folder);
            report::json(&body, exit)
        }
        Output::Human => {
            let said = audit_message::human((&audit, &weights), &trend, &folder);
            report::text(&said, exit)
        }
    }
}

/// Builds the site into `site/` in `folder` and opens it as a crawl
/// source. A page left from an earlier build would be audited as if it
/// were still there, so the folder is made again each time. A failure is
/// printed in `output` and comes back as its exit.
fn built(
    site: &Path,
    checked: (&Config, &Routes, &SiteContent),
    folder: &Path,
    clock: &dyn Clock,
    output: Output,
) -> Result<Folder, Exit> {
    let built = within(folder, "site");
    if built.exists()
        && let Err(error) = fs::remove_dir_all(&built)
    {
        let message = audit_message::write(&built, &error);
        return Err(report::failure(output, &message, Exit::Failure));
    }
    build::into_folder(site, checked, &built, clock, output)?;
    let home = home(checked.0)
        .map_err(|()| report::failure(output, &audit_message::internal(), Exit::Internal))?;
    Folder::open(&built, &home)
        .map_err(|error| report::failure(output, &audit_message::of(&error), Exit::Internal))
}

/// The audit's limits, measured on today's date from `clock`. The answer
/// length, the paragraph length and the stale window come from the site's
/// `lint.toml`, so the linter and the audit judge a post alike.
fn thresholds(site: &Path, clock: &dyn Clock) -> Result<Thresholds, UserMessage> {
    let lint = lint::lint_settings(site)?.thresholds();
    Ok(Thresholds::default()
        .with_answer_words(lint.answer_min_words, lint.answer_max_words)
        .with_paragraph_words(lint.paragraph_max_words)
        .with_stale_months(lint.stale_months)
        .with_today(clock.now().date_naive()))
}

/// The blog's home page: the site's address with its mount.
fn home(config: &Config) -> Result<AbsoluteUrl, ()> {
    let address = config.address().as_str().trim_end_matches('/');
    AbsoluteUrl::parse(&format!("{address}{}", config.mount())).map_err(drop)
}

/// Writes the issues, the link report and the page weights as CSV into
/// `folder`, adds this run to the history, and returns every run, oldest
/// first.
fn keep(
    folder: &Path,
    audit: &Report,
    (links, weights): (&LinkReport, &WeightReport),
    clock: &dyn Clock,
) -> Result<Vec<Run>, UserMessage> {
    fs::create_dir_all(folder).map_err(|error| audit_message::write(folder, &error))?;
    let write = |name: &str, text: &str| {
        let file = within(folder, name);
        fs::write(&file, text).map_err(|error| audit_message::write(&file, &error))
    };
    write("issues.csv", &audit.csv())?;
    write("links.csv", &links.csv())?;
    write("weights.csv", &weights.csv())?;
    let line = Run::of(audit, clock.now())
        .line()
        .map_err(|_| audit_message::internal())?;
    // Appended, never rewritten, so a failed write loses this run at most.
    let file = within(folder, "runs.jsonl");
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file)
        .and_then(|mut runs| runs.write_all(line.as_bytes()))
        .map_err(|error| audit_message::write(&file, &error))?;
    let runs = fs::read_to_string(&file).map_err(|error| audit_message::write(&file, &error))?;
    Ok(history(&runs))
}

/// `relative` inside `folder`; every `relative` here is a fixed name.
fn within(folder: &Path, relative: &str) -> PathBuf {
    let mut path = folder.to_path_buf();
    path.push(relative);
    path
}
