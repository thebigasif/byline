//! `khalid watch`: how fast and how reliably the blog answered at its
//! public address over the last day and the last 30 days, from the looks
//! `khalid serve` keeps (C6). A long gap between looks is time nobody
//! watched, as when the server itself was down.

use std::fmt::Write as _;
use std::path::Path;

use chrono::{DateTime, Duration, SecondsFormat, Utc};
use khalid_alerts::{Answer, Probe};
use khalid_core::{Clock, Slug};
use khalid_db::{Database, DbError, WatchTable};
use serde::Serialize;

use crate::cli::Output;
use crate::db::MAIN;
use crate::exit::Exit;
use crate::site_watch::KEPT_DAYS;
use crate::{db, db_message, report};

/// Said when the watch has no looks yet.
const NO_LOOKS: &str = "The watch has not looked at the blog yet.\nkhalid serve looks at the public address every few minutes once it answers more than this computer.\nSet how often with watch_minutes in [alerts] of site.toml.\n";

/// The JSON shape of `khalid watch --json`.
#[derive(Serialize)]
struct WatchBody {
    pages: Vec<PageReport>,
}

/// One page's looks, over two spans.
#[derive(Serialize)]
struct PageReport {
    page: String,
    last_day: Span,
    last_30_days: Span,
    last_look: Option<Look>,
}

/// The looks of one span of time.
#[derive(Serialize)]
struct Span {
    looks: usize,
    answered: usize,
    middle_ms: Option<u32>,
    slowest_ms: Option<u32>,
    longest_gap_minutes: i64,
}

/// The newest look. `status` is `null` when nothing answered.
#[derive(Serialize)]
struct Look {
    at: String,
    status: Option<u16>,
    millis: u32,
}

/// Prints the report for the `site` folder's database at the time `clock`
/// gives.
pub(crate) fn run(site: &Path, clock: &dyn Clock, output: Output) -> Exit {
    let path = match db::data_file(site) {
        Ok(path) => path,
        Err(message) => return report::failure(output, &message, Exit::Failure),
    };
    let (Ok(main), Ok(runtime)) = (
        Slug::try_new(MAIN),
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build(),
    ) else {
        return report::failure(output, &db_message::internal(), Exit::Internal);
    };
    let now = clock.now();
    match runtime.block_on(pages(&path, &main, now)) {
        Ok(pages) if output == Output::Json => report::json(&WatchBody { pages }, Exit::Success),
        Ok(pages) if pages.is_empty() => report::text(NO_LOOKS, Exit::Success),
        Ok(pages) => report::text(&human(&pages), Exit::Success),
        Err(error) => report::failure(output, &db_message::of(&path, &error), Exit::Failure),
    }
}

/// Every watched page's report at `now`, from the database at `path`.
async fn pages(path: &Path, handle: &Slug, now: DateTime<Utc>) -> Result<Vec<PageReport>, DbError> {
    let db = Database::open(path).await?;
    let read = async {
        db.migrate().await?;
        let table = WatchTable::new(db.clone(), db.site(handle).await?);
        let mut reports = Vec::new();
        for page in table.pages().await? {
            let looks = table.since(&page, now - Duration::days(KEPT_DAYS)).await?;
            let day: Vec<Probe> = looks
                .iter()
                .filter(|look| look.at > now - Duration::days(1))
                .copied()
                .collect();
            reports.push(PageReport {
                last_day: span(&day, now),
                last_30_days: span(&looks, now),
                last_look: looks.last().map(look),
                page,
            });
        }
        Ok(reports)
    };
    let reports = read.await;
    db.close().await;
    reports
}

/// What `looks`, oldest first, show up to `now`.
fn span(looks: &[Probe], now: DateTime<Utc>) -> Span {
    let mut times: Vec<u32> = looks
        .iter()
        .filter(|look| look.is_up())
        .map(|look| look.millis)
        .collect();
    times.sort_unstable();
    let ends = looks.iter().map(|look| look.at).chain([now]);
    let gaps = ends.clone().zip(ends.skip(1)).map(|(a, b)| b - a);
    Span {
        looks: looks.len(),
        answered: times.len(),
        middle_ms: times.get(times.len() / 2).copied(),
        slowest_ms: times.last().copied(),
        longest_gap_minutes: gaps.max().map_or(0, |gap| gap.num_minutes()),
    }
}

/// The newest look as the report shows it.
fn look(probe: &Probe) -> Look {
    let status = match probe.answer {
        Answer::Status(status) => Some(status),
        Answer::Unreached => None,
    };
    Look {
        at: probe.at.to_rfc3339_opts(SecondsFormat::Secs, true),
        status,
        millis: probe.millis,
    }
}

/// The report in sentences, one block per page.
fn human(pages: &[PageReport]) -> String {
    let mut text = String::new();
    for page in pages {
        let _ = writeln!(text, "{}", page.page);
        let _ = writeln!(text, "  Last 24 hours: {}", said(&page.last_day));
        let _ = writeln!(text, "  Last 30 days: {}", said(&page.last_30_days));
        if let Some(last) = &page.last_look {
            let got = last.status.map_or_else(
                || "got no answer".to_owned(),
                |status| format!("got status {status}"),
            );
            let _ = writeln!(
                text,
                "  Last look: {}, {got} in {} ms.",
                last.at, last.millis
            );
        }
    }
    text
}

/// One span in a sentence, such as `12 looks, 100.0% answered, middle
/// 120 ms, slowest 300 ms, longest gap 5 minutes.`
fn said(span: &Span) -> String {
    let Some(per_mille) = span.answered.saturating_mul(1000).checked_div(span.looks) else {
        return "no looks.".to_owned();
    };
    let share = format!("{}.{}%", per_mille / 10, per_mille % 10);
    let times = match (span.middle_ms, span.slowest_ms) {
        (Some(middle), Some(slowest)) => format!(", middle {middle} ms, slowest {slowest} ms"),
        _ => String::new(),
    };
    let gap = span.longest_gap_minutes;
    format!(
        "{} looks, {share} answered{times}, longest gap {gap} minutes.",
        span.looks
    )
}
