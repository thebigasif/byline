//! `khalid search-console email`: the weekly report in plain words, sent
//! to the address in `report_to` the way `[mail]` says (plan 5.11). A
//! timer the owner sets up runs it once a week.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use chrono::NaiveDate;
use khalid_audit::history;
use khalid_content::SiteContent;
use khalid_core::{Config, Environment, UserMessage};
use khalid_newsletter::{EmailAddress, Mail};
use khalid_outbound::Outbound;
use khalid_search_console::{Question, SearchConsole, go_deeper};

use crate::bing::{Failed, block_on};
use crate::cli::Output;
use crate::exit::Exit;
use crate::search_console_report::{ReportBody, gather};
use crate::{newsletter, report, serve_message};

/// Gathers the week's numbers for the checked `site` and mails them to
/// `report_to`, with the mail secret from `env`.
pub(crate) fn run(
    site: (&Path, &Config, &SiteContent),
    env: &Environment,
    asked: (&SearchConsole, &Outbound, NaiveDate),
    output: Output,
) -> Result<Exit, Failed> {
    let (folder, config, _) = site;
    let report_to = config
        .search_console()
        .and_then(|settings| settings.report_to.as_deref());
    let (Some(to), Some(mail)) = (report_to, config.mail()) else {
        return Err((no_one(), Exit::Failure));
    };
    let to = EmailAddress::parse(to).map_err(|_| (no_one(), Exit::Failure))?;
    let secret = env.mail_secret(&mail.transport);
    let mailer = newsletter::mailer(config, secret, asked.1)
        .ok()
        .flatten()
        .ok_or_else(|| (serve_message::internal(), Exit::Internal))?;
    let body = gather(site, asked)?;
    let subject = format!("{}: your week in Google search", config.name());
    let text = weekly(config, &body, &score(folder));
    let letter = Mail::new(to.clone(), subject, text);
    block_on(mailer.send(&letter))?.map_err(|_| (not_sent(), Exit::Failure))?;
    let sent = format!("The weekly Search Console email went to {to}.\n");
    Ok(match output {
        Output::Json => report::json(
            &serde_json::json!({ "sent_to": to.to_string() }),
            Exit::Success,
        ),
        Output::Human => report::text(&sent, Exit::Success),
    })
}

/// The site's health score from its last `khalid audit`, in a sentence.
fn score(folder: &Path) -> String {
    let mut runs = folder.to_path_buf();
    runs.push("data/audit/runs.jsonl");
    let last = fs::read_to_string(&runs)
        .ok()
        .and_then(|text| history(&text).pop());
    match last {
        Some(run) => format!(
            "Site health score: {} out of 100, from the audit on {}.",
            run.score,
            run.at.date_naive()
        ),
        None => "No site audit has run yet. Run khalid audit for the health score.".to_owned(),
    }
}

/// The weekly email's text for the site with `config`.
fn weekly(config: &Config, body: &ReportBody, score: &str) -> String {
    let mut text = format!(
        "This is {}'s week in Google search, up to {}, the last day Google has final numbers for.\n\n\
         Clicks from Google: {:.0} this week, against {:.0} the week before.\n{score}\n",
        config.name(),
        body.last,
        body.week_clicks,
        body.last_week_clicks
    );
    // Writing to a String cannot fail, so the results are dropped.
    for (heading, list) in [
        ("Gained the most clicks:", &body.winners),
        ("Lost the most clicks:", &body.losers),
    ] {
        if !list.is_empty() {
            let _ = write!(text, "\n{heading}\n");
        }
        for change in list {
            let _ = writeln!(
                text,
                "- {}: {:.0} clicks, from {:.0}.",
                change.title, change.now, change.before
            );
            if let Some(link) = &change.go_deeper {
                let _ = writeln!(text, "  Why? Go deeper in SerpDad: {link}");
            }
        }
    }
    text.push_str("\nWhat to do next:\n");
    let mut tasks = 0;
    for decay in &body.decaying {
        tasks += 1;
        let _ = writeln!(
            text,
            "- Refresh \"{}\": its clicks fell three months in a row.\n  Why? Go deeper in SerpDad: {}",
            decay.title, decay.go_deeper
        );
    }
    for page in &body.lost {
        tasks += 1;
        let _ = match page.not_found_hits {
            Some(_) => writeln!(
                text,
                "- Add a redirect for {}: it answers 404, not found, but still gets search traffic.",
                page.url
            ),
            None => writeln!(
                text,
                "- Check {}: it is marked noindex, but still gets search traffic.",
                page.url
            ),
        };
    }
    if tasks == 0 {
        text.push_str("- Nothing needs fixing this week.\n");
    }
    let next = go_deeper(Question::WhatToWriteNext, config.address().as_str());
    let _ = write!(
        text,
        "\nWhat should you write next? Go deeper in SerpDad: {next}\n\n\
         Khalid sent this because report_to under [search_console] in site.toml names you.\n"
    );
    text
}

/// The message when no one is named to get the email.
fn no_one() -> UserMessage {
    UserMessage::new(
        "No one is named to get the weekly Search Console email.",
        "Khalid sends it only to the address in report_to under [search_console], and only through [mail].",
        "Add report_to = \"you@example.com\" under [search_console], and a [mail] table, in site.toml.",
    )
}

/// The message when the mail transport did not take the email.
fn not_sent() -> UserMessage {
    UserMessage::new(
        "The weekly Search Console email could not be sent.",
        "The way [mail] in site.toml names did not take it.",
        "Check [mail] and its secret as docs/deploy.md shows under Sending email, then run the command again.",
    )
}
