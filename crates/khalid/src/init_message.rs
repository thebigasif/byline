//! What a person reads when `khalid init` cannot make a site (RULES.md BL-CLI-3).

use std::io;
use std::path::Path;

use khalid_core::UserMessage;

/// The message when `--address` is missing and Khalid may not ask for it.
pub(crate) fn needs_address() -> UserMessage {
    UserMessage::new(
        "Khalid needs the website's address to set up a site.",
        "The address has no default, because every link on the blog starts with it.",
        "Run `khalid init --address https://www.example.com` with your own address.",
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
