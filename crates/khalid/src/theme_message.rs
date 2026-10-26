//! What a person reads when theme sync cannot finish (RULES.md BL-ERR-4).

use khalid_core::{AbsoluteUrl, UserMessage};
use khalid_outbound::OutboundError;
use khalid_theme::{FetchError, TokensError};

/// Turns a failure to read the main site's home page at `url` into what, why
/// and fix.
pub(crate) fn fetch(url: &AbsoluteUrl, error: &FetchError) -> UserMessage {
    match error {
        FetchError::Unreachable {
            source: OutboundError::Blocked { address, .. },
        } => UserMessage::new(
            format!(
                "Khalid did not read {url}, because it leads to a private address, {address}: \
                 one that only machines inside a network can reach."
            ),
            "Theme sync reads public websites only, so a link cannot reach machines behind your firewall.",
            "Give the main site's public address, such as https://www.example.com.",
        ),
        _ => UserMessage::new(
            format!("Khalid could not read {url}."),
            "The address may be wrong, or the site may be down, so nothing has changed.",
            "Open the address in a browser to check it, then run `khalid theme sync` again.",
        ),
    }
}

/// Turns a failure to write `theme/tokens.toml` into what, why and fix.
pub(crate) fn tokens(error: &TokensError) -> UserMessage {
    let what = match error {
        TokensError::Write { path, .. } => format!("Khalid could not write {}.", path.display()),
        _ => "Khalid could not save the theme in the site folder.".to_owned(),
    };
    UserMessage::new(
        what,
        "The theme was read, but the blog keeps its old colours and fonts until the file is saved.",
        "Check that you can write to the theme folder of the site, then run it again.",
    )
}

/// The message when the web client or the runtime cannot start.
pub(crate) fn internal() -> UserMessage {
    UserMessage::new(
        "Khalid could not start its web client.",
        "The cause is inside Khalid or the computer, not in the address you gave.",
        "Run the command again, and report it with the output of `khalid --version` if it fails again.",
    )
}
