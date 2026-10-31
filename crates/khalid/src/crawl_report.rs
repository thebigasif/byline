//! `khalid crawl health` and `errors`: what the visitor log says about
//! crawling (GEO C3, C5). They read `data/site.db`, and answer that the
//! log is off when `[visitor_log]` does not switch it on.

use std::path::Path;

use chrono::{DateTime, Days, SecondsFormat, Utc};
use khalid_analytics::{
    AnalyticsError, Answers, BotError, CrawlerHealth, VisitorLogStore, bot_errors, crawl_health,
};
use khalid_core::{Clock, Environment, UserMessage};
use khalid_db::VisitorLogTable;
use serde::Serialize;

use crate::cli::{CrawlCommand, Output};
use crate::exit::Exit;
use crate::{crawl_words, db_message, report, serve, site_config};

/// What one command read.
enum Read {
    Health(Vec<CrawlerHealth>),
    Errors(Vec<BotError>),
}

/// Runs a crawl `command` on the `site` folder, reading the visitor log's
/// kept days up to now by `clock`.
pub(crate) fn run(site: &Path, command: CrawlCommand, clock: &dyn Clock, output: Output) -> Exit {
    let config = match site_config::load(site, &Environment::from_process()) {
        Ok(config) => config,
        Err(message) => return report::failure(output, &message, Exit::Failure),
    };
    let settings = config.visitor_log();
    if !settings.on {
        return report::failure(output, &crawl_words::log_off(), Exit::Failure);
    }
    let now = clock.now();
    let since = now
        .checked_sub_days(Days::new(u64::from(settings.keep_days)))
        .unwrap_or(DateTime::<Utc>::MIN_UTC);
    match read(site, command, (since, now)) {
        Ok(read) => match output {
            Output::Json => report::json(&json(&read), Exit::Success),
            Output::Human => {
                let days = u32::from(settings.keep_days);
                report::text(&text(&read, days), Exit::Success)
            }
        },
        Err(message) => report::failure(output, &message, Exit::Failure),
    }
}

/// Reads what `command` needs from the site's visitor log.
fn read(
    site: &Path,
    command: CrawlCommand,
    (since, now): (DateTime<Utc>, DateTime<Utc>),
) -> Result<Read, UserMessage> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| db_message::internal())?;
    let (database, id) = serve::open(site, &runtime)?;
    runtime.block_on(async {
        let log = VisitorLogTable::new(database.clone(), id);
        let read = async {
            Ok::<_, AnalyticsError>(match command {
                CrawlCommand::Health => {
                    let visits = log.visits(since, now).await?;
                    Read::Health(crawl_health(&visits, &log.crawled().await?))
                }
                CrawlCommand::Errors => Read::Errors(bot_errors(&log.visits(since, now).await?)),
            })
        }
        .await;
        database.close().await;
        read.map_err(|_| crawl_words::unreadable())
    })
}

/// The JSON shape of every command: one list under its name.
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum Body<'a> {
    Crawlers(Vec<CrawlerBody>),
    Errors(Vec<ErrorBody<'a>>),
}

#[derive(Serialize)]
struct CrawlerBody {
    crawler: &'static str,
    kind: &'static str,
    listed: AnswersBody,
    unchecked: AnswersBody,
    pretending: u64,
    pages: u64,
    last_at: Option<String>,
}

#[derive(Serialize)]
struct AnswersBody {
    ok: u64,
    moved: u64,
    missing: u64,
    failed: u64,
}

#[derive(Serialize)]
struct ErrorBody<'a> {
    path: &'a str,
    status: u16,
    crawlers: Vec<&'static str>,
    hits: u64,
    last_at: String,
}

/// `at` as JSON shows it: RFC 3339 in UTC, to the second.
pub(crate) fn time(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn answers(of: Answers) -> AnswersBody {
    AnswersBody {
        ok: of.ok,
        moved: of.moved,
        missing: of.missing,
        failed: of.failed,
    }
}

fn json(read: &Read) -> Body<'_> {
    match read {
        Read::Health(all) => Body::Crawlers(
            all.iter()
                .map(|one| CrawlerBody {
                    crawler: one.crawler.name(),
                    kind: one.crawler.kind().as_str(),
                    listed: answers(one.listed),
                    unchecked: answers(one.unchecked),
                    pretending: one.pretending,
                    pages: one.pages,
                    last_at: one.last_at.map(time),
                })
                .collect(),
        ),
        Read::Errors(errors) => Body::Errors(
            errors
                .iter()
                .map(|error| ErrorBody {
                    path: &error.path,
                    status: error.status,
                    crawlers: error.crawlers.iter().map(|bot| bot.name()).collect(),
                    hits: error.hits,
                    last_at: time(error.last_at),
                })
                .collect(),
        ),
    }
}

fn text(read: &Read, days: u32) -> String {
    match read {
        Read::Health(all) => crawl_words::health(all, days),
        Read::Errors(errors) => crawl_words::errors(errors, days),
    }
}

/// A time as the sentences show it, to the minute, such as
/// `2026-11-03 09:00 UTC`.
pub(crate) fn shown(at: DateTime<Utc>) -> String {
    at.format("%Y-%m-%d %H:%M UTC").to_string()
}
