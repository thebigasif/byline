//! `khalid bing`: hands Bing Webmaster Tools the site's sitemap, and reads
//! how Bing crawls the site, with the key in `KHALID_BING_API_KEY` (task
//! C8, ADR "Bing Webmaster Tools through its API key").

use khalid_core::{Config, Environment, UserMessage};
use khalid_outbound::Outbound;
use khalid_seo::{BingError, BingWebmaster, sitemap_index_url};
use serde::Serialize;

use crate::cli::{BingCommand, Cli, Output};
use crate::exit::Exit;
use crate::{bing_report, build_message, build_seo, outbound, report, serve_message};

/// A message and the exit code it ends with.
pub(crate) type Failed = (UserMessage, Exit);

/// The JSON shape of a submitted sitemap.
#[derive(Serialize)]
struct SubmittedBody<'a> {
    submitted: &'a str,
}

/// Runs `command` for the site with `config`, with the key in `env`.
pub(crate) fn run(config: &Config, env: &Environment, command: BingCommand, cli: &Cli) -> Exit {
    let output = cli.output;
    let done = connect(config, env, cli).and_then(|(bing, outbound)| match command {
        BingCommand::Sitemap => sitemap(config, &bing, &outbound, output),
        BingCommand::Report => bing_report::run(config, &bing, &outbound, output),
    });
    done.unwrap_or_else(|(message, exit)| report::failure(output, &message, exit))
}

/// Hands Bing the site's sitemap index, which lists every sitemap file.
fn sitemap(
    config: &Config,
    bing: &BingWebmaster,
    outbound: &Outbound,
    output: Output,
) -> Result<Exit, Failed> {
    let site = build_seo::site(config).map_err(|message| (message, Exit::Failure))?;
    let index =
        sitemap_index_url(&site).map_err(|error| (build_message::seo(&error), Exit::Failure))?;
    block_on(bing.submit_sitemap(outbound, &index))?.map_err(|error| failed(config, &error))?;
    Ok(match output {
        Output::Json => report::json(
            &SubmittedBody {
                submitted: index.as_str(),
            },
            Exit::Success,
        ),
        Output::Human => report::text(
            &format!(
                "Bing Webmaster Tools has the sitemap at {index}.\n\
                 Bing reads the site's pages from it over the next few days.\n"
            ),
            Exit::Success,
        ),
    })
}

/// The API for the site's address, with the key from `env`, and the one
/// outbound client.
fn connect(
    config: &Config,
    env: &Environment,
    cli: &Cli,
) -> Result<(BingWebmaster, Outbound), Failed> {
    let key = env
        .bing_api_key()
        .ok_or_else(|| (no_key(), Exit::Failure))?;
    let bing = BingWebmaster::new(config.address().clone(), key)
        .map_err(|error| failed(config, &error))?;
    #[cfg(feature = "test-util")]
    let bing = match &cli.test_bing {
        Some(endpoint) => bing.with_endpoint(endpoint),
        None => bing,
    };
    let outbound =
        outbound::client(cli).map_err(|_| (serve_message::internal(), Exit::Internal))?;
    Ok((bing, outbound))
}

/// Runs `work` to its end on a runtime of its own.
pub(crate) fn block_on<T>(work: impl Future<Output = T>) -> Result<T, Failed> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| (serve_message::internal(), Exit::Internal))?;
    Ok(runtime.block_on(work))
}

/// What, why and fix for `error`, about the site with `config`.
pub(crate) fn failed(config: &Config, error: &BingError) -> Failed {
    let message = match error {
        BingError::Key => UserMessage::new(
            "The key in KHALID_BING_API_KEY is not one Bing Webmaster Tools makes.",
            "Bing's keys hold only letters and digits, so Bing would refuse this one.",
            "Copy the key again from Bing Webmaster Tools, under Settings, API Access.",
        ),
        BingError::KeyRefused => UserMessage::new(
            "Bing Webmaster Tools does not know the API key.",
            "The key in KHALID_BING_API_KEY was deleted in Bing, or copied with a mistake.",
            "Make a new key in Bing Webmaster Tools, under Settings, API Access, and set it as KHALID_BING_API_KEY.",
        ),
        BingError::NotVerified => UserMessage::new(
            format!(
                "Bing Webmaster Tools has not verified {} for this key.",
                config.address()
            ),
            "Bing answers only about a site its owner proved is theirs.",
            "Add the site in Bing Webmaster Tools, put its code as bing under [seo.verification] in site.toml, and verify.",
        ),
        BingError::Refused { status, reason } => UserMessage::new(
            format!("Bing Webmaster Tools turned the request down ({status}, {reason})."),
            "Bing does this when a key sends too many requests, among other reasons.",
            "Wait an hour, then run the command again.",
        ),
        BingError::Request { failure } => UserMessage::new(
            format!("The request to Bing Webmaster Tools did not complete: {failure}."),
            "Khalid could not reach Bing from this computer.",
            "Check that this computer can reach ssl.bing.com, then run the command again.",
        ),
        BingError::Address => UserMessage::new(
            "The site's address could not be put into a request to Bing Webmaster Tools.",
            "The address in site.toml holds something a request cannot carry.",
            "Check address in site.toml, then run the command again.",
        ),
        _ => UserMessage::new(
            "Bing Webmaster Tools answered with something Khalid cannot read.",
            "Bing may have changed its API, or something on the way answered for it.",
            "Run the command again later, and report it to the Khalid project if it keeps happening.",
        ),
    };
    (message, Exit::Failure)
}

/// The message when no key is set.
fn no_key() -> UserMessage {
    UserMessage::new(
        "Khalid has no Bing Webmaster Tools API key, the password a program uses to reach Bing for you.",
        "Bing answers only to the key of the person who verified the site there.",
        "Make one in Bing Webmaster Tools, under Settings, API Access, and set it as KHALID_BING_API_KEY in this computer's environment.",
    )
}
