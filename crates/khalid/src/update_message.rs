//! What a person reads when `khalid update` cannot finish (RULES.md
//! BL-ERR-4).

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
