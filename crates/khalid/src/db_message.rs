//! What a person reads when the database cannot be used (RULES.md BL-ERR-4).

use std::io;
use std::path::Path;

use khalid_core::UserMessage;
use khalid_db::DbError;

/// Turns a database error on the file at `path` into what, why and fix.
pub(crate) fn of(path: &Path, error: &DbError) -> UserMessage {
    let path = path.display();
    match error {
        DbError::Open { .. } => UserMessage::new(
            format!("Khalid could not open the database {path}."),
            "The database holds the site's index and everything visitors create.",
            format!("Check that you can write to {path}, or restore it from a backup."),
        ),
        DbError::Migrate { .. } => UserMessage::new(
            format!("Khalid could not update the database {path}."),
            "A newer Khalid, or a change by hand, left it in a state this version does not know.",
            format!("Use the Khalid version that last opened {path}, or restore it from a backup."),
        ),
        DbError::Unreadable { column, .. } => UserMessage::new(
            format!("The database {path} holds a {column} that Khalid did not write."),
            "Something other than Khalid changed the index, so it cannot be trusted.",
            "Run `khalid db rebuild` to write the index again from your files.",
        ),
        _ => UserMessage::new(
            format!("A step in the database {path} failed."),
            "Khalid stopped so that nothing is left half written.",
            "Check the disk has free space and nothing else holds the file, then run it again.",
        ),
    }
}

/// The message when the `data/` folder cannot be made.
pub(crate) fn folder(folder: &Path, error: &io::Error) -> UserMessage {
    let folder = folder.display();
    let why = match error.kind() {
        io::ErrorKind::PermissionDenied => "You do not have permission to write there.",
        _ => "Something else, such as a file with the same name, is in the way.",
    };
    UserMessage::new(
        format!("Khalid could not make the folder {folder}."),
        why,
        format!("Remove or rename what is at {folder}, or fix its permissions, then run it again."),
    )
}

/// The message when the async runtime cannot start, which leaves nothing the
/// person typed to blame.
pub(crate) fn runtime() -> UserMessage {
    UserMessage::new(
        "Khalid could not start its background workers.",
        "The computer refused the threads or files Khalid needs to run.",
        "Close other programs and run the command again.",
    )
}
