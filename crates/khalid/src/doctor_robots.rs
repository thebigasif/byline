//! The check `khalid doctor` runs on the host's own `robots.txt` for a blog
//! in a folder: fetch it through `Outbound` and confirm it names the blog's
//! sitemap, since search engines read `robots.txt` only at the root of the
//! host (KH-SEO-5, ADR "root files go in their own folder").

use khalid_core::{AbsoluteUrl, Config, UserMessage};
use khalid_outbound::OutboundError;

use crate::build_files::ROOT_FOLDER;
use crate::cli::{Cli, Network};
use crate::doctor_check::{Check, Finding};
use crate::outbound;

/// Fetches `/robots.txt` on the blog's host, unless the blog is at the root
/// of it, `network` says to stay offline, or the settings did not load.
pub(crate) fn check(config: Option<&Config>, network: Network, cli: &Cli) -> Finding {
    let Some(config) = config else {
        return Finding::skipped(
            Check::Robots,
            "Doctor reads the host from the settings, so fix those first.",
        );
    };
    if config.mount() == "/" {
        return Finding::skipped(
            Check::Robots,
            "The blog is at the root of its host, so its own robots.txt is the one search engines read.",
        );
    }
    if network == Network::Offline {
        return Finding::skipped(
            Check::Robots,
            "You asked doctor to stay offline with --offline, so it did not fetch robots.txt.",
        );
    }
    let host = config.address().as_str().trim_end_matches('/');
    let sitemap = format!("{host}{}sitemap.xml", config.mount());
    let (Ok(url), Ok(client), Ok(runtime)) = (
        AbsoluteUrl::parse(&format!("{host}/robots.txt")),
        outbound::client(cli),
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build(),
    ) else {
        return Finding::problem(Check::Robots, internal());
    };
    match runtime.block_on(client.get(&url)) {
        Err(OutboundError::Blocked { .. }) => Finding::problem(Check::Robots, private(&url)),
        Err(_) => Finding::problem(Check::Robots, unreachable(&url)),
        Ok(file) if !(200..300).contains(&file.status()) => Finding::problem(
            Check::Robots,
            UserMessage::new(
                format!("{url} answers with the error {}.", file.status()),
                "Search engines find no sitemap line there, so they may miss the blog's posts.",
                copy_fix(),
            ),
        ),
        Ok(file) if !names(&String::from_utf8_lossy(file.body()), &sitemap) => Finding::problem(
            Check::Robots,
            UserMessage::new(
                format!("{url} does not name the blog's sitemap."),
                "Search engines read the sitemap lines only there, so they may miss the blog's posts.",
                format!(
                    "Add the line `Sitemap: {sitemap}` to it, or copy the lines from {ROOT_FOLDER}/robots.txt in the build folder, then run `khalid doctor` again."
                ),
            ),
        ),
        Ok(_) => Finding::ok(Check::Robots, format!("{url} names the blog's sitemap.")),
    }
}

/// Whether `robots` has a `Sitemap:` line for `sitemap`. The field name is
/// read in any case, as crawlers read it.
fn names(robots: &str, sitemap: &str) -> bool {
    robots.lines().any(|line| {
        line.split_once(':').is_some_and(|(field, value)| {
            field.trim().eq_ignore_ascii_case("sitemap") && value.trim() == sitemap
        })
    })
}

/// The fix when the host has no `robots.txt` the blog can use.
fn copy_fix() -> String {
    format!(
        "Copy robots.txt from {ROOT_FOLDER}/ in the build folder to the root of the main site, then run `khalid doctor` again."
    )
}

/// The message for a host on a private network, which the outbound guard
/// refuses before anything is sent.
fn private(url: &AbsoluteUrl) -> UserMessage {
    UserMessage::new(
        format!("{url} is on a private network, so doctor did not fetch it."),
        "Khalid fetches only public addresses, the ones search engines can reach.",
        "Run doctor once the site is live at its public address, and use --offline until then.",
    )
}

/// The message for a host that did not answer in time, or at all.
fn unreachable(url: &AbsoluteUrl) -> UserMessage {
    UserMessage::new(
        format!("Khalid could not reach {url}."),
        "Search engines cannot reach it either, so they do not see the blog's sitemap.",
        "Check that the site is online and that `address` in site.toml is right, then run `khalid doctor` again.",
    )
}

/// The message when the fetch cannot start for a reason inside Khalid or
/// the computer.
fn internal() -> UserMessage {
    UserMessage::new(
        "Khalid could not start fetching the host's robots.txt.",
        "The cause is inside Khalid or the computer, not in your site.",
        "Run `khalid doctor` again, and report it with the output of `khalid --version` if it fails again.",
    )
}

#[cfg(test)]
mod tests {
    use super::names;

    #[test]
    fn a_sitemap_line_is_found_in_any_case_and_spacing() {
        let sitemap = "https://www.example.com/blog/sitemap.xml";
        assert!(names(&format!("sitemap:{sitemap}\n"), sitemap));
        assert!(names(
            &format!("User-agent: *\nSITEMAP :  {sitemap}  \n"),
            sitemap
        ));
        assert!(!names(
            "Sitemap: https://www.example.com/sitemap.xml\n",
            sitemap
        ));
        assert!(!names(&format!("# Sitemap: {sitemap}\n"), sitemap));
    }
}
