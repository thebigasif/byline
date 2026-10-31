//! What a person reads when `khalid update` cannot finish (RULES.md
//! BL-ERR-4).

use std::fmt::Display;
use std::io;
use std::path::Path;

use khalid_core::UserMessage;

/// The message when Khalid cannot tell where its own binary is.
pub(crate) fn no_binary(error: &io::Error) -> UserMessage {
    UserMessage::new(
        format!("Khalid could not find its own program file: {error}."),
        "An update replaces that file, so Khalid must know where it is.",
        "Run Khalid by its full path, such as /usr/local/bin/khalid, and try again.",
    )
}

/// The message when no release is built for this system.
pub(crate) fn no_system() -> UserMessage {
    UserMessage::new(
        format!(
            "Khalid has no release built for {} on {}.",
            std::env::consts::OS,
            std::env::consts::ARCH
        ),
        "Releases are built for Linux, macOS on Apple silicon and Windows, so there is nothing to fetch.",
        "Build the newest version from source with `cargo install --git https://github.com/thebigasif/khalid khalid`.",
    )
}

/// The message when the release list at `url` names no file for `platform`.
pub(crate) fn no_file(platform: &str, url: &impl Display) -> UserMessage {
    UserMessage::new(
        format!("The newest release has no file for {platform}."),
        format!("Its list of files, {url}, names none for this system."),
        "Check https://github.com/thebigasif/khalid/releases, and try again once the release is complete.",
    )
}

/// The message when `url` cannot be fetched, and `cause` says why.
pub(crate) fn unreachable(url: &str, cause: &str) -> UserMessage {
    UserMessage::new(
        format!("Khalid could not fetch {url}: {cause}."),
        "Khalid reads its releases there, so it cannot update now, and nothing changed.",
        "Check the computer is online, then run `khalid update` again.",
    )
}

/// The message when the update cannot start for a reason inside Khalid or
/// the computer.
pub(crate) fn internal() -> UserMessage {
    UserMessage::new(
        "Khalid could not start the update.",
        "The cause is inside Khalid or the computer, not in anything you typed.",
        "Run the command again, and report it with the output of `khalid --version` if it fails again.",
    )
}

/// The message when no earlier version is kept beside `binary`.
pub(crate) fn nothing_kept(binary: &Path) -> UserMessage {
    UserMessage::new(
        format!(
            "There is no earlier version of Khalid beside {}.",
            binary.display()
        ),
        "An update keeps the version before it, and no update has run from this file yet.",
        "Install the version you want from https://github.com/thebigasif/khalid/releases instead.",
    )
}

/// The message when the binary files cannot be moved.
pub(crate) fn swap(binary: &Path, error: &io::Error) -> UserMessage {
    let folder = binary.parent().map_or_else(
        || binary.display().to_string(),
        |folder| folder.display().to_string(),
    );
    let why = match error.kind() {
        io::ErrorKind::PermissionDenied => {
            format!("You do not have permission to change files in {folder}.")
        }
        _ => format!("A file in {folder} could not be moved, so the version in use stays."),
    };
    UserMessage::new(
        format!(
            "Khalid could not change the program file {}.",
            binary.display()
        ),
        why,
        "Run the command again as the user who installed Khalid, such as with sudo.",
    )
}
