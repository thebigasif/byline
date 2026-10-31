//! `khalid analytics ai`: how many readers each AI assistant sent, against
//! the days before (GEO M1), through the same `AnalyticsDesk` the Admin API
//! reads.

use std::fmt::Write;
use std::path::Path;
use std::sync::Arc;

use chrono::NaiveDate;
use khalid_analytics::{AiVisitors, Period, ToolVisitors};
use khalid_api::analytics_desk::AnalyticsDesk;
use khalid_core::{Slug, SystemClock, UserMessage};
use khalid_db::{AnalyticsTable, Database};
use serde::Serialize;

use crate::cli::Output;
use crate::exit::Exit;
use crate::{db, db_message, report};

/// The JSON shape of `khalid analytics ai --json`, as the Admin API
/// answers it.
#[derive(Serialize)]
struct ReportBody {
    first: NaiveDate,
    last: NaiveDate,
    days: u32,
    arrivals: u64,
    ai_views: u64,
    tools: Vec<ToolBody>,
}

#[derive(Serialize)]
struct ToolBody {
    tool: &'static str,
    name: &'static str,
    views: u64,
    before: u64,
    pages: Vec<PageBody>,
}

#[derive(Serialize)]
struct PageBody {
    path: String,
    views: u64,
}

/// Reports the readers AI assistants sent to the `site` folder in the
/// last `days` days. Only the database is read.
pub(crate) fn run(site: &Path, days: u32, output: Output) -> Exit {
    match (read(site, days), output) {
        (Ok(report), Output::Json) => report::json(&body(report), Exit::Success),
        (Ok(report), Output::Human) => report::text(&human(&report), Exit::Success),
        (Err(message), _) => report::failure(output, &message, Exit::Failure),
    }
}

/// Opens the site's database, brings it up to date and reads the report.
fn read(site: &Path, days: u32) -> Result<AiVisitors, UserMessage> {
    let path = db::data_file(site)?;
    let (Ok(main), Ok(runtime)) = (
        Slug::try_new(db::MAIN),
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build(),
    ) else {
        return Err(db_message::internal());
    };
    runtime.block_on(async {
        let database = Database::open(&path)
            .await
            .map_err(|error| db_message::of(&path, &error))?;
        let opened = async {
            database.migrate().await?;
            database.site(&main).await
        }
        .await;
        let done = match opened {
            Ok(id) => {
                let table = Arc::new(AnalyticsTable::new(database.clone(), id));
                let desk = AnalyticsDesk::new(table, Arc::new(SystemClock));
                let period = Period::ending(desk.today(), days).ok_or_else(bad_days)?;
                desk.ai_visitors(period).await.map_err(|_| unreadable())
            }
            Err(error) => Err(db_message::of(&path, &error)),
        };
        database.close().await;
        done
    })
}

fn body(report: AiVisitors) -> ReportBody {
    let tool = |row: ToolVisitors| ToolBody {
        tool: row.tool.key(),
        name: row.tool.name(),
        views: row.views,
        before: row.before,
        pages: row
            .pages
            .into_iter()
            .map(|page| PageBody {
                path: page.path,
                views: page.views,
            })
            .collect(),
    };
    ReportBody {
        first: report.period.first(),
        last: report.period.last(),
        days: report.period.days(),
        arrivals: report.arrivals,
        ai_views: report.ai_views,
        tools: report.tools.into_iter().map(tool).collect(),
    }
}

/// The report in plain words: the share first, then each assistant with
/// its pages.
fn human(report: &AiVisitors) -> String {
    let period = report.period;
    let days = period.days();
    let mut text = format!(
        "Readers sent by AI assistants, {} to {} ({days} days)\n",
        period.first(),
        period.last()
    );
    if report.tools.is_empty() {
        text.push_str(
            "No reader came from an AI assistant such as ChatGPT or Perplexity in these days.\n\
             Views are counted while khalid serve runs, so a site served another way shows none.\n",
        );
        return text;
    }
    let share = report
        .ai_views
        .saturating_mul(100)
        .checked_div(report.arrivals)
        .unwrap_or(0);
    let _ = writeln!(
        text,
        "{} of {} views from outside the site came from AI assistants ({share}%).",
        report.ai_views, report.arrivals
    );
    for tool in &report.tools {
        let _ = write!(
            text,
            "\n{}: {} views, {} in the {days} days before\n",
            tool.tool.name(),
            tool.views,
            tool.before
        );
        for page in &tool.pages {
            let _ = writeln!(text, "  {}  {} views", page.path, page.views);
        }
    }
    text.push_str("\nA view is a page opened, not a person. AI bots are never counted here.\n");
    text
}

fn bad_days() -> UserMessage {
    UserMessage::new(
        "The days are not a number from 1 to 366.",
        "The report counts back from today, at most a year.",
        "Run it with --days 28, or leave --days out.",
    )
}

fn unreadable() -> UserMessage {
    UserMessage::new(
        "Khalid could not read the site's views in data/site.db.",
        "Without them there is nothing to report.",
        "Check nothing else holds data/site.db, then run it again.",
    )
}
