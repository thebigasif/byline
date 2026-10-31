//! `khalid analytics uncited`: the pages AI bots read that no AI assistant
//! sent a reader to (GEO M2, KH-GRO-4). It reads the counts `khalid serve`
//! keeps in `data/site.db`, so it shows nothing for a site only ever built
//! as static files.

use std::fmt::Write;
use std::path::Path;

use chrono::{DateTime, Days, NaiveDate, SecondsFormat, Utc};
use khalid_analytics::{AnalyticsStore, Uncited, uncited};
use khalid_core::{Clock, UserMessage};
use khalid_db::AnalyticsTable;
use serde::Serialize;

use crate::cli::Output;
use crate::exit::Exit;
use crate::{db_message, report, serve};

/// The JSON shape of `khalid analytics uncited --json`.
#[derive(Serialize)]
struct UncitedBody<'a> {
    days: u32,
    pages: Vec<PageBody<'a>>,
}

/// One page as `--json` prints it.
#[derive(Serialize)]
struct PageBody<'a> {
    path: &'a str,
    reads: u64,
    bots: &'a [String],
    last_read: DateTime<Utc>,
}

impl<'a> From<&'a Uncited> for PageBody<'a> {
    fn from(page: &'a Uncited) -> Self {
        Self {
            path: &page.path,
            reads: page.reads,
            bots: &page.bots,
            last_read: page.last_read,
        }
    }
}

/// Runs `analytics uncited` on the `site` folder, for the last `days` up
/// to today by `clock`. Only its database is read, not its content.
pub(crate) fn run(site: &Path, days: u32, clock: &dyn Clock, output: Output) -> Exit {
    let last = clock.now().date_naive();
    let first = last
        .checked_sub_days(Days::new(u64::from(days.saturating_sub(1))))
        .unwrap_or(NaiveDate::MIN);
    match read(site, first, last) {
        Ok(pages) => match output {
            Output::Json => {
                let pages = pages.iter().map(PageBody::from).collect();
                report::json(&UncitedBody { days, pages }, Exit::Success)
            }
            Output::Human => report::text(&text(&pages, days), Exit::Success),
        },
        Err(message) => report::failure(output, &message, Exit::Failure),
    }
}

/// The pages read but never cited from `first` to `last`, both included.
fn read(site: &Path, first: NaiveDate, last: NaiveDate) -> Result<Vec<Uncited>, UserMessage> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| db_message::internal())?;
    let (database, id) = serve::open(site, &runtime)?;
    runtime.block_on(async {
        let table = AnalyticsTable::new(database.clone(), id);
        let counts = async {
            let crawls = table.crawls(first, last).await?;
            let views = table.views(first, last).await?;
            Ok::<_, khalid_analytics::AnalyticsError>(uncited(&crawls, &views))
        }
        .await;
        database.close().await;
        counts.map_err(|_| unreadable())
    })
}

/// What a person reads: each page with its reads, then what to do.
fn text(pages: &[Uncited], days: u32) -> String {
    let period = if days == 1 {
        "today".to_owned()
    } else {
        format!("the last {days} days")
    };
    if pages.is_empty() {
        return format!(
            "No page was read by AI bots without being cited in {period}. \
             Either AI bots read no page, or every page they read also had a reader \
             sent by an AI assistant.\n"
        );
    }
    let (count, them) = match pages.len() {
        1 => ("1 page was".to_owned(), "it"),
        many => (format!("{many} pages were"), "them"),
    };
    let mut lines = format!(
        "{count} read by AI bots in {period}, but no AI assistant sent a reader to {them}.\n\n"
    );
    for page in pages {
        let last = page.last_read.to_rfc3339_opts(SecondsFormat::Secs, true);
        let reads = match page.reads {
            1 => "1 read".to_owned(),
            many => format!("{many} reads"),
        };
        let (path, bots) = (&page.path, page.bots.join(", "));
        let _ = writeln!(lines, "{path}  {reads} by {bots}, last {last}");
    }
    lines.push_str(
        "\nA bot reading a page is not a citation: only a reader arriving from an AI assistant is. \
         Bots are named by the user agent they send, which anyone can copy.\n\
         To be cited, start each section with a short, direct answer to the question its heading asks. \
         Run `khalid lint` to see where a page's answers are missing or too long.\n",
    );
    lines
}

/// The answer when the counts in `data/site.db` cannot be read.
fn unreadable() -> UserMessage {
    UserMessage::new(
        "Khalid could not read the site's counts in data/site.db.",
        "Without them it cannot tell which pages AI bots read.",
        "Check nothing else holds data/site.db and the disk is not full, then run it again.",
    )
}
