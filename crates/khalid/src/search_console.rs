//! `khalid search-console`: the site's Google Search Console numbers, read
//! with the owner's own Google account once `[search_console]` in
//! `site.toml` opts in (KH-GRO-5, ADR "Search Console through the owner's
//! refresh token").

use std::path::Path;

use chrono::{Days, NaiveDate};
use khalid_content::SiteContent;
use khalid_core::{Clock, Config, Environment, SystemClock, UserMessage};
use khalid_outbound::Outbound;
use khalid_search_console::{Credentials, SearchConsole, SearchConsoleError, Window};
use serde::Serialize;

use crate::bing::{Failed, block_on};
use crate::cli::{Cli, Output, SearchConsoleCommand};
use crate::exit::Exit;
use crate::{outbound, report, search_console_report, serve_message};

/// Days Google takes to make a day's numbers final.
const SETTLING_DAYS: u64 = 3;

/// The days a month of numbers covers: four whole weeks, so each month
/// holds every weekday as often.
const MONTH_DAYS: u32 = 28;

/// The JSON shape of a check that passed.
#[derive(Serialize)]
struct CheckBody<'a> {
    property: &'a str,
    first: String,
    last: String,
    pages: usize,
}

/// Runs `command` for the checked `site`, with the Google secrets in
/// `env`.
pub(crate) fn run(
    site: (&Path, &Config, &SiteContent),
    env: &Environment,
    command: SearchConsoleCommand,
    cli: &Cli,
) -> Exit {
    let (_, config, _) = site;
    let output = cli.output;
    let done = connect(config, env, cli).and_then(|(console, outbound)| {
        let asked = (&console, &outbound, SystemClock.now().date_naive());
        match command {
            SearchConsoleCommand::Check => check(asked, output),
            SearchConsoleCommand::Report => search_console_report::run(site, asked, output),
        }
    });
    done.unwrap_or_else(|(message, exit)| report::failure(output, &message, exit))
}

/// Signs in and reads the last month's pages, to show the connection works.
fn check(
    (console, outbound, today): (&SearchConsole, &Outbound, NaiveDate),
    output: Output,
) -> Result<Exit, Failed> {
    let month = last_month(today);
    let pages = ask(console, async {
        console.sign_in(outbound).await?.pages(month).await
    })?;
    let body = CheckBody {
        property: console.property(),
        first: month.first.to_string(),
        last: month.last.to_string(),
        pages: pages.len(),
    };
    Ok(match output {
        Output::Json => report::json(&body, Exit::Success),
        Output::Human => report::text(
            &format!(
                "Khalid can read the Search Console property {}.\n\
                 Google counted searches for {} of its pages from {} to {}.\n",
                body.property, body.pages, body.first, body.last
            ),
            Exit::Success,
        ),
    })
}

/// The last 28 days before `today` whose numbers are final, or the day
/// itself when the calendar runs out.
pub(crate) fn last_month(today: NaiveDate) -> Window {
    let last = today
        .checked_sub_days(Days::new(SETTLING_DAYS))
        .unwrap_or(today);
    Window::ending(last, MONTH_DAYS).unwrap_or(Window { first: last, last })
}

/// The connection `[search_console]` names, with the secrets from `env`,
/// and the one outbound client.
fn connect(
    config: &Config,
    env: &Environment,
    cli: &Cli,
) -> Result<(SearchConsole, Outbound), Failed> {
    let settings = config
        .search_console()
        .ok_or_else(|| (not_opted_in(), Exit::Failure))?;
    let (Some(client_secret), Some(refresh_token)) =
        (env.google_client_secret(), env.google_refresh_token())
    else {
        return Err((no_secrets(), Exit::Failure));
    };
    let credentials = Credentials {
        client_id: settings.client_id.clone(),
        client_secret,
        refresh_token,
    };
    let console = SearchConsole::new(&settings.property, credentials);
    #[cfg(feature = "test-util")]
    let console = match &cli.test_google {
        Some(endpoint) => console.with_endpoint(endpoint),
        None => console,
    };
    let outbound =
        outbound::client(cli).map_err(|_| (serve_message::internal(), Exit::Internal))?;
    Ok((console, outbound))
}

/// Runs `work` against `console`'s property, and turns its error into
/// what, why and fix.
pub(crate) fn ask<T>(
    console: &SearchConsole,
    work: impl Future<Output = Result<T, SearchConsoleError>>,
) -> Result<T, Failed> {
    block_on(work)?.map_err(|error| failed(console.property(), &error))
}

/// What, why and fix for `error`, about `property`.
fn failed(property: &str, error: &SearchConsoleError) -> Failed {
    let message = match error {
        SearchConsoleError::TokenRefused => UserMessage::new(
            "Google no longer accepts the refresh token in KHALID_GOOGLE_REFRESH_TOKEN.",
            "The token stops working when the Google account takes back Khalid's permission, or after six months unused.",
            "Make a new refresh token as docs/deploy.md shows under Google Search Console, and set it again.",
        ),
        SearchConsoleError::ClientRefused => UserMessage::new(
            "Google does not know the OAuth client, the app identity Khalid signs in as.",
            "client_id in [search_console] or KHALID_GOOGLE_CLIENT_SECRET does not match a client in Google Cloud.",
            "Copy both again from Google Cloud, under APIs and Services, Credentials.",
        ),
        SearchConsoleError::NotAllowed => UserMessage::new(
            format!("The Google account may not read the Search Console property {property}."),
            "Google answers only for a property the signed-in account owns or was added to.",
            "Check property in [search_console] against Search Console, or add the account to the property there.",
        ),
        SearchConsoleError::Quota => UserMessage::new(
            "Google's quota for these requests is spent for now.",
            "Google allows a set number of requests a minute and a day for each property.",
            "Wait a minute, or until tomorrow for the daily quota, then run the command again.",
        ),
        SearchConsoleError::Request { .. } => UserMessage::new(
            "The request to Google did not complete.",
            "Khalid could not reach Google from this computer.",
            "Check that this computer can reach googleapis.com, then run the command again.",
        ),
        _ => UserMessage::new(
            "Google answered with something Khalid cannot read.",
            "Google may have changed its API, or something on the way answered for it.",
            "Run the command again later, and report it to the Khalid project if it keeps happening.",
        ),
    };
    (message, Exit::Failure)
}

/// The message when `site.toml` has no `[search_console]`.
fn not_opted_in() -> UserMessage {
    UserMessage::new(
        "This site is not connected to Google Search Console, Google's report of how a site does in search.",
        "Khalid asks Google nothing until the owner opts in under [search_console] in site.toml.",
        "Follow the steps in docs/deploy.md under Google Search Console, then run the command again.",
    )
}

/// The message when a Google secret is missing.
fn no_secrets() -> UserMessage {
    UserMessage::new(
        "Khalid has no Google sign-in for Search Console.",
        "Google answers only with the OAuth client's secret and the owner's refresh token, a long-lived permission to read.",
        "Set KHALID_GOOGLE_CLIENT_SECRET and KHALID_GOOGLE_REFRESH_TOKEN in this computer's environment, as docs/deploy.md shows.",
    )
}
