//! What a person reads when theme sync cannot finish (RULES.md BL-ERR-4).

use std::io;
use std::path::Path;

use khalid_core::{AbsoluteUrl, SitePathError, UserMessage};
use khalid_outbound::OutboundError;
use khalid_theme::{FetchError, FontHost, HeldFont, MAX_CSS_BYTES, TokensError};

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
        FetchError::Status { status } => UserMessage::new(
            format!("{url} answered with status {status} instead of a page."),
            "Theme sync takes the colours and fonts from that page, so nothing has changed.",
            "Open the address in a browser to check it shows the site, then run it again.",
        ),
        FetchError::TooLarge { limit }
        | FetchError::Unreachable {
            source: OutboundError::TooLarge { limit },
        } => UserMessage::new(
            format!(
                "The page at {url} is larger than the {} MiB theme sync reads.",
                limit / (1024 * 1024)
            ),
            "Khalid stops there to keep a sync fast and safe, so nothing has changed.",
            "Give the address of a smaller page on the same site that uses the same stylesheets.",
        ),
        _ => UserMessage::new(
            format!("Khalid could not reach {url}."),
            "The name may be misspelt, the site may be down, or this computer may be offline, so nothing has changed.",
            format!(
                "Open the address in a browser to check it, then run `khalid theme sync {url}` again."
            ),
        ),
    }
}

/// The message when the page at `url` uses no CSS Khalid could read.
pub(crate) fn no_stylesheet(url: &AbsoluteUrl, skipped: usize) -> UserMessage {
    let why = if skipped == 0 {
        "The colours and fonts come from the site's CSS, so there is nothing to sync and nothing has changed."
    } else {
        "Its stylesheets are on another site or could not be read, and the colours and fonts come from them, so nothing has changed."
    };
    UserMessage::new(
        format!("Khalid found no stylesheet on {url}."),
        why,
        "Give the address of a page on the main site that shows its usual colours, such as the home page.",
    )
}

/// What a person reads when the page at `url` is drawn by JavaScript, so
/// the blog took the clean default look.
pub(crate) fn drawn_by_script(url: &AbsoluteUrl) -> String {
    format!(
        "{url} draws its pages with JavaScript, so its HTML shows almost nothing to copy. \
         The blog uses the clean default colours and fonts for now. \
         To use your site's own, save its CSS variables in a file and run \
         `khalid theme sync {url} --from-css <file>`."
    )
}

/// What a person reads about a font the blog may not serve.
pub(crate) fn held_font(font: &HeldFont) -> String {
    let service = match font.host() {
        FontHost::Adobe => "Adobe Fonts",
        _ => font.server(),
    };
    format!(
        "The font {} comes from {service}, whose licence does not let the blog serve it, \
         so the blog shows the next font in its list.",
        font.family()
    )
}

/// The message when the CSS file given with `--from-css` cannot be read.
pub(crate) fn css_file(file: &Path, error: &io::Error) -> UserMessage {
    let what = if error.kind() == io::ErrorKind::FileTooLarge {
        format!(
            "Khalid did not read {}, because it is larger than the {} MiB theme sync reads.",
            file.display(),
            MAX_CSS_BYTES / (1024 * 1024)
        )
    } else {
        format!("Khalid could not read {}: {error}.", file.display())
    };
    UserMessage::new(
        what,
        "Theme sync takes the colours and fonts from that file, so nothing has changed.",
        "Save the site's CSS variables as a plain text file, check the path, then run it again.",
    )
}

/// Turns a failure to write `theme/tokens.toml` into what, why and fix.
pub(crate) fn tokens(error: &TokensError) -> UserMessage {
    match error {
        TokensError::Path {
            source: SitePathError::EscapesRoot { .. },
        } => UserMessage::new(
            "The theme folder in this site is a link to a place outside the site folder.",
            "Khalid writes only inside the site folder, so nothing has changed.",
            "Replace that link with a real folder named theme, then run it again.",
        ),
        TokensError::Write { path, .. } => UserMessage::new(
            format!("Khalid could not write {}.", path.display()),
            "The theme was read, but the blog keeps its old colours and fonts until the file is saved.",
            "Check that you can write to the theme folder and the disk has space, then run it again.",
        ),
        _ => UserMessage::new(
            "Khalid could not save the theme in theme/tokens.toml.",
            "The theme was read, but the blog keeps its old colours and fonts until the file is saved.",
            "Check that theme in the site folder is a folder you can write to, then run it again.",
        ),
    }
}

/// Turns a failure to read the `theme/tokens.toml` already in the site into
/// what, why and fix. A token that is missing or misspelt is a mistake on
/// its line.
pub(crate) fn read(error: &TokensError) -> UserMessage {
    match error {
        TokensError::Parse { line, source } => {
            let at = line.map_or_else(String::new, |line| format!(" on line {line}"));
            let detail = source.to_string();
            let detail = detail.lines().rfind(|l| !l.trim().is_empty());
            UserMessage::new(
                format!(
                    "theme/tokens.toml has a mistake{at}: {}.",
                    detail.unwrap_or("a value the blog cannot read")
                ),
                "Each of the twelve tokens must be there with a value the blog can use, so Khalid changed nothing.",
                "Fix that line, or delete the file to take every value from the main site again, then run it again.",
            )
        }
        TokensError::Path {
            source: SitePathError::EscapesRoot { .. },
        } => tokens(error),
        _ => UserMessage::new(
            "Khalid could not read theme/tokens.toml.",
            "It holds the values you set by hand, so Khalid changed nothing.",
            "Check that theme in the site folder is a folder you can read and write, then run it again.",
        ),
    }
}

/// The message when the web client or the runtime cannot start.
pub(crate) fn internal() -> UserMessage {
    UserMessage::new(
        "Khalid could not start its web client.",
        "The cause is inside Khalid or the computer, not in the address you gave.",
        "Run the command again, and report it with the output of `khalid --version` if it fails again.",
    )
}
