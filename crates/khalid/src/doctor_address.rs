//! The check `khalid doctor` runs on the blog's public address: fetch it
//! through `Outbound` and confirm Khalid answers there, which proves the
//! proxy rule on the main site works (PRD KH-INS-3, RULES.md BL-RUN-8).

use khalid_core::{AbsoluteUrl, Config, UserMessage};
use khalid_outbound::OutboundError;

use crate::assets::FOLDER;
use crate::cli::{Cli, Network};
use crate::doctor_check::{Check, Finding};
use crate::outbound;

/// Fetches the address `config` gives the blog, unless `network` says to
/// stay offline or the settings did not load.
pub(crate) fn check(config: Option<&Config>, network: Network, cli: &Cli) -> Finding {
    let Some(config) = config else {
        return Finding::skipped(
            Check::Address,
            "Doctor fetches the address in the settings, so fix those first.",
        );
    };
    if network == Network::Offline {
        return Finding::skipped(
            Check::Address,
            "You asked doctor to stay offline with --offline, so it did not fetch the blog.",
        );
    }
    let base = config.address().as_str().trim_end_matches('/');
    let Ok(url) = AbsoluteUrl::parse(&format!("{base}{}", config.mount())) else {
        return Finding::problem(Check::Address, no_address(config));
    };
    let (Ok(client), Ok(runtime)) = (
        outbound::client(cli),
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build(),
    ) else {
        return Finding::problem(Check::Address, internal());
    };
    // Khalid's pages link the stylesheet from the mount's assets folder,
    // which no other server's page does.
    let marker = format!("href=\"{}{FOLDER}site.", config.mount());
    let mount = config.mount();
    match runtime.block_on(client.get(&url)) {
        Err(OutboundError::Blocked { .. }) => Finding::problem(Check::Address, private(&url)),
        Err(_) => Finding::problem(Check::Address, unreachable(&url)),
        Ok(page) if !(200..300).contains(&page.status()) => Finding::problem(
            Check::Address,
            UserMessage::new(
                format!(
                    "{url} answers with the error {} instead of the blog.",
                    page.status()
                ),
                "Readers and search engines that follow a link to the blog get that error.",
                proxy_fix(mount),
            ),
        ),
        Ok(page) if !String::from_utf8_lossy(page.body()).contains(&marker) => Finding::problem(
            Check::Address,
            UserMessage::new(
                format!("{url} answers with a page Khalid did not make."),
                "Another server answers there, so readers never see your posts.",
                proxy_fix(mount),
            ),
        ),
        Ok(_) => Finding::ok(Check::Address, format!("The blog answers at {url}.")),
    }
}

/// The fix when something other than the blog answers under `mount`.
fn proxy_fix(mount: &str) -> String {
    format!(
        "Set up the proxy rule, the main site's setting that sends {mount} to Khalid, then run `khalid doctor` again."
    )
}

/// The message for an address on a private network, which the outbound
/// guard refuses before anything is sent.
fn private(url: &AbsoluteUrl) -> UserMessage {
    UserMessage::new(
        format!("{url} is on a private network, so doctor did not fetch it."),
        "Khalid fetches only public addresses, the ones readers and search engines can reach.",
        "Run doctor once the blog is live at its public address, and use --offline until then.",
    )
}

/// The message for an address that did not answer in time, or at all.
fn unreachable(url: &AbsoluteUrl) -> UserMessage {
    UserMessage::new(
        format!("Khalid could not reach {url}."),
        "Readers and search engines cannot reach it either, so they never see the blog.",
        "Check that the site is online and that `address` in site.toml is right, then run `khalid doctor` again.",
    )
}

/// The message when the address and the mount do not make one address.
fn no_address(config: &Config) -> UserMessage {
    UserMessage::new(
        format!(
            "The address {} and the mount {} do not make one web address.",
            config.address(),
            config.mount()
        ),
        "Doctor needs the blog's full address to fetch it.",
        "Correct `address` and `mount` in site.toml, then run `khalid doctor` again.",
    )
}

/// The message when the fetch cannot start for a reason inside Khalid or
/// the computer.
fn internal() -> UserMessage {
    UserMessage::new(
        "Khalid could not start fetching the blog's public address.",
        "The cause is inside Khalid or the computer, not in your site.",
        "Run `khalid doctor` again, and report it with the output of `khalid --version` if it fails again.",
    )
}
