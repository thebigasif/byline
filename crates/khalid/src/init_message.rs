//! What a person reads when `khalid init` cannot make a site (RULES.md BL-CLI-3).

use std::io;
use std::path::Path;

use khalid_core::{ConfigError, UserMessage};

/// The message when the folder already holds a site.
pub(crate) fn exists(settings: &Path) -> UserMessage {
    UserMessage::new(
        format!("{} already exists.", settings.display()),
        "Khalid never overwrites a site's settings, so nothing has changed.",
        "Choose an empty folder, or edit site.toml by hand to change the settings.",
    )
}

/// The message when the answers do not make valid settings. It names the
/// flag that gave the wrong value.
pub(crate) fn refused(error: &ConfigError) -> UserMessage {
    let (field, problem) = match error {
        ConfigError::Field { field, problem, .. } => (field.as_str(), problem.as_str()),
        ConfigError::Parse { field, source, .. } => (field.as_str(), source.message()),
        _ => ("", "the answers do not make valid settings"),
    };
    let flag = match field {
        "address" => "--address",
        "mount" => "--mount",
        "mode" => "--mode",
        "languages" | "default_language" => "--languages",
        _ => "the flags",
    };
    UserMessage::new(
        format!(
            "Khalid cannot use the value for {flag}: {}.",
            problem.trim_end_matches('.')
        ),
        "Khalid checks every setting before it writes site.toml, so nothing was written.",
        format!("Run `khalid init` again with a corrected {flag}."),
    )
}

/// The message when `--address` is missing and Khalid may not ask for it.
pub(crate) fn needs_address() -> UserMessage {
    UserMessage::new(
        "Khalid needs the website's address to set up a site.",
        "The address has no default, because every link on the blog starts with it.",
        "Run `khalid init --address https://www.example.com` with your own address.",
    )
}

/// The message when the questions stopped before the last answer.
pub(crate) fn no_answer() -> UserMessage {
    UserMessage::new(
        "Khalid stopped before every question was answered.",
        "A site needs all four answers, so nothing was written.",
        "Run `khalid init` again, or give the answers as flags.",
    )
}

/// The message when a file or folder of the new site cannot be written.
pub(crate) fn write(folder: &Path, error: &io::Error) -> UserMessage {
    UserMessage::new(
        format!(
            "Khalid could not write the site in {}: {error}.",
            folder.display()
        ),
        "The site is not complete until every file and folder exists.",
        "Check that you may write to that folder, then run `khalid init` again.",
    )
}

/// The message when the mount is not a folder Khalid can give to a stack.
pub(crate) fn mount(mount: &str) -> UserMessage {
    UserMessage::new(
        format!("Khalid could not turn {mount} into a folder path."),
        "The proxy rule names the folder of your website the blog lives under.",
        "Write it as a path such as /blog/, or use / for the whole website.",
    )
}

/// The message when the blog's own address could not be built.
pub(crate) fn address(address: &str) -> UserMessage {
    UserMessage::new(
        format!("Khalid could not make the blog's address {address}."),
        "The proxy rule sends the blog's folder to the address Khalid answers on.",
        "Pass the blog's own address with --blog, such as https://blog.example.com.",
    )
}

/// The message for a stack there is no rule for.
pub(crate) fn stack(name: &str) -> UserMessage {
    UserMessage::new(
        format!("There is no proxy rule for {name}."),
        "Khalid knows the rule for the stacks a main site is usually built with.",
        "Use one of: nginx, caddy, apache, traefik, cloudflare-worker, vercel, nextjs, netlify, webflow, shopify.",
    )
}
