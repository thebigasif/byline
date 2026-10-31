//! What a person reads when `khalid backup` cannot finish (RULES.md
//! BL-ERR-4).

use std::io;
use std::path::Path;

use khalid_core::UserMessage;

/// The message when the folder holds no `site.toml`.
pub(crate) fn not_a_site(site: &Path) -> UserMessage {
    UserMessage::new(
        format!(
            "{} has no site.toml, so it is not a site folder.",
            site.display()
        ),
        "Khalid backs up one site, and site.toml marks the folder that holds it.",
        "Run `khalid --site <folder> backup` with the folder that holds site.toml.",
    )
}

/// The message when a file is already where the archive would go.
pub(crate) fn exists(archive: &Path) -> UserMessage {
    UserMessage::new(
        format!(
            "{} is already there, so Khalid wrote no backup.",
            archive.display()
        ),
        "Khalid never writes over a file, so an older backup is never lost.",
        "Pick another name with --out, or move the old file away first.",
    )
}

/// The message when the archive cannot be written, or a site file read.
pub(crate) fn write(archive: &Path, error: &io::Error) -> UserMessage {
    let why = match error.kind() {
        io::ErrorKind::PermissionDenied => {
            "You do not have permission to read a site file or write there."
        }
        io::ErrorKind::NotFound => "The folder the archive goes in does not exist.",
        _ => "The disk may be full, or a file changed while Khalid read it.",
    };
    UserMessage::new(
        format!("Khalid could not write the backup {}.", archive.display()),
        why,
        "Fix that, then run `khalid backup` again.",
    )
}
