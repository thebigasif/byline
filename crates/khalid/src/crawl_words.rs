//! The words of `khalid crawl`: its reports as sentences, and its
//! messages, each with what, why and fix.

use std::fmt::Write as _;

use khalid_analytics::{Answers, BotError, CrawlerHealth};
use khalid_core::UserMessage;

use crate::crawl_report::shown;

/// The most error lines a person is shown; `--json` has every one.
const MOST_ERRORS: usize = 50;

/// What a checked crawler is, said once under each report.
pub(crate) const CHECKED: &str = "A crawler counts only when its address is on the list its company publishes. \
     Anyone can send its name, so a visit from another address is never counted as it.\n";

/// Each search and AI crawler's visits over the kept `days`.
pub(crate) fn health(all: &[CrawlerHealth], days: u32) -> String {
    if all.is_empty() {
        return format!(
            "No search or AI crawler has visited in the last {days} days the visitor log keeps.\n"
        );
    }
    let mut lines = format!("Search and AI crawlers over the last {days} days.\n\n");
    for one in all {
        let name = one.crawler.name();
        let kind = one.crawler.kind().as_str();
        let _ = writeln!(lines, "{name} ({kind})");
        let _ = writeln!(lines, "  From its own addresses: {}", said(one.listed));
        if one.unchecked != Answers::default() {
            let _ = writeln!(lines, "  With no list to check: {}", said(one.unchecked));
        }
        let pages = match one.pages {
            1 => "1 address".to_owned(),
            many => format!("{many} addresses"),
        };
        let last = one.last_at.map_or_else(|| "never".to_owned(), shown);
        let _ = writeln!(lines, "  {pages} fetched, last visit {last}");
        if one.pretending > 0 {
            let _ = writeln!(
                lines,
                "  {} from other addresses using its name, not counted",
                visits(one.pretending)
            );
        }
    }
    lines.push('\n');
    lines.push_str(CHECKED);
    lines
}

/// The addresses that answered crawlers with an error over the kept `days`.
pub(crate) fn errors(errors: &[BotError], days: u32) -> String {
    if errors.is_empty() {
        return format!(
            "No search or AI crawler got an error in the last {days} days the visitor log keeps.\n"
        );
    }
    let mut lines = format!(
        "Addresses that answered search and AI crawlers with an error in the last {days} days, most first.\n\n"
    );
    for error in errors.iter().take(MOST_ERRORS) {
        let names: Vec<&str> = error.crawlers.iter().map(|bot| bot.name()).collect();
        let _ = writeln!(
            lines,
            "{}  {}  {} to {}, last {}",
            error.path,
            error.status,
            times(error.hits),
            names.join(", "),
            shown(error.last_at)
        );
    }
    if let Some(more) = errors
        .len()
        .checked_sub(MOST_ERRORS)
        .filter(|more| *more > 0)
    {
        let _ = writeln!(
            lines,
            "... and {more} more. Run it with --json to see every one."
        );
    }
    lines.push_str(
        "\nA 404 on an address that should work needs a redirect: run khalid redirects. \
         A 5xx is a server error: read the server log for it.\n",
    );
    lines
}

fn said(answers: Answers) -> String {
    format!(
        "{} with the page, {} moved, {} missing, {} failed",
        answers.ok, answers.moved, answers.missing, answers.failed
    )
}

fn visits(count: u64) -> String {
    match count {
        1 => "1 visit".to_owned(),
        many => format!("{many} visits"),
    }
}

fn times(count: u64) -> String {
    match count {
        1 => "once".to_owned(),
        many => format!("{many} times"),
    }
}

/// The answer when the visitor log is off.
pub(crate) fn log_off() -> UserMessage {
    UserMessage::new(
        "The visitor log is off, so Khalid has no crawler visits to report.",
        "Crawl reports read the visitor log, which a site keeps only once it switches it on.",
        "Read docs/design/visitor-log.md, then add [visitor_log] with on = true to site.toml and run khalid serve.",
    )
}

/// The answer when the visitor log in `data/site.db` cannot be read.
pub(crate) fn unreadable() -> UserMessage {
    UserMessage::new(
        "Khalid could not read the visitor log in data/site.db.",
        "Without it, it cannot tell which crawlers came or what they got.",
        "Check nothing else holds data/site.db and the disk is not full, then run it again.",
    )
}
