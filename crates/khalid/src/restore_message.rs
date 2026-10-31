//! What a person reads when `khalid restore` cannot finish (RULES.md
//! BL-ERR-4).

use std::io;
use std::path::Path;

use khalid_core::UserMessage;

/// The message when the folder to restore into is not new or empty.
pub(crate) fn not_empty(folder: &Path) -> UserMessage {
    UserMessage::new(
        format!(
            "{} already holds files, so Khalid restored nothing.",
            folder.display()
        ),
        "Khalid never unpacks over a site, so a live site and its database are never mixed with an old copy.",
        "Restore into a new folder, or stop the site and move its folder away first.",
    )
}

/// The message when the archive cannot be opened or read.
pub(crate) fn read(archive: &Path, error: &io::Error) -> UserMessage {
    let why = match error.kind() {
        io::ErrorKind::NotFound => "There is no file there.",
        io::ErrorKind::PermissionDenied => "You do not have permission to read it.",
        _ => "It is not a .tar.gz archive, or it is damaged.",
    };
    UserMessage::new(
        format!("Khalid could not read the backup {}.", archive.display()),
        why,
        "Give the path of a .tar.gz file that `khalid backup` wrote.",
    )
}

/// The message when an entry sits outside the archive's one site folder,
/// or is not a plain file or folder.
pub(crate) fn outside(archive: &Path, entry: &Path) -> UserMessage {
    UserMessage::new(
        format!(
            "The backup {} holds {}, outside its one site folder, so Khalid restored nothing.",
            archive.display(),
            entry.display()
        ),
        "`khalid backup` keeps plain files in one folder, so this archive was made another way or changed.",
        "Restore an archive that `khalid backup` wrote.",
    )
}

/// The message when the archive holds no `site.toml`.
pub(crate) fn not_a_backup(archive: &Path) -> UserMessage {
    UserMessage::new(
        format!(
            "The backup {} holds no site.toml, so Khalid restored nothing.",
            archive.display()
        ),
        "Every site keeps its settings in site.toml, so this archive is not a site backup.",
        "Restore an archive that `khalid backup` wrote.",
    )
}

/// The message when the site cannot be written.
pub(crate) fn write(folder: &Path, error: &io::Error) -> UserMessage {
    let why = match error.kind() {
        io::ErrorKind::PermissionDenied => "You do not have permission to write there.",
        io::ErrorKind::NotFound => "The folder it goes in does not exist.",
        _ => "The disk may be full, or another program changed the folder meanwhile.",
    };
    UserMessage::new(
        format!(
            "Khalid could not restore the site into {}.",
            folder.display()
        ),
        why,
        "Fix that, then run `khalid restore` again. Nothing was left half restored.",
    )
}
