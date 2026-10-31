//! `khalid restore`: makes a site again from an archive `khalid backup`
//! wrote (plan section 4.1, KH-OPS-2). The site goes into a folder that is
//! new or empty, so a live site and its database are never written over.

use std::ffi::OsString;
use std::fs::{self, File};
use std::io;
use std::path::{Component, Path, PathBuf};

use flate2::read::GzDecoder;
use khalid_core::UserMessage;
use serde::Serialize;
use tar::EntryType;
use tempfile::TempDir;

use crate::cli::Output;
use crate::exit::Exit;
use crate::{report, restore_message};

/// The JSON shape of `khalid restore --json`.
#[derive(Serialize)]
struct RestoreBody {
    folder: String,
    files: usize,
    database: bool,
}

/// What came out of an archive.
struct Restored {
    files: usize,
    database: bool,
}

/// Unpacks `archive` into `folder`, which must be new or empty.
pub(crate) fn run(archive: &Path, folder: &Path, output: Output) -> Exit {
    if holds_files(folder) {
        return report::failure(output, &restore_message::not_empty(folder), Exit::Failure);
    }
    match unpack(archive, folder) {
        Ok(restored) => success(output, folder, &restored),
        Err(message) => report::failure(output, &message, Exit::Failure),
    }
}

/// Whether `folder` is there and is anything but an empty folder.
fn holds_files(folder: &Path) -> bool {
    folder.exists() && fs::read_dir(folder).map_or(true, |mut entries| entries.next().is_some())
}

/// Unpacks every file into a temporary folder beside `folder`, then moves
/// that into place, so a restore is whole or absent.
fn unpack(archive: &Path, folder: &Path) -> Result<Restored, UserMessage> {
    let read = |error: io::Error| restore_message::read(archive, &error);
    let written = |error: io::Error| restore_message::write(folder, &error);
    let file = File::open(archive).map_err(read)?;
    let beside = match folder.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => PathBuf::from("."),
    };
    let temporary = tempfile::Builder::new()
        .prefix(".khalid-restore-")
        .tempdir_in(&beside)
        .map_err(written)?;
    let mut tar = tar::Archive::new(GzDecoder::new(file));
    let mut top: Option<OsString> = None;
    let mut restored = Restored {
        files: 0,
        database: false,
    };
    let mut settings = false;
    for entry in tar.entries().map_err(read)? {
        let mut entry = entry.map_err(read)?;
        let path = entry.path().map_err(read)?.into_owned();
        let outside = || restore_message::outside(archive, &path);
        let (name, inside) = split(&path).ok_or_else(outside)?;
        if top.get_or_insert_with(|| name.clone()) != &name {
            return Err(outside());
        }
        let target = under(&temporary, &inside);
        match entry.header().entry_type() {
            EntryType::Directory => fs::create_dir_all(&target).map_err(written)?,
            EntryType::Regular | EntryType::Continuous if inside.components().next().is_some() => {
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent).map_err(written)?;
                }
                entry.unpack(&target).map_err(written)?;
                restored.files += 1;
                settings |= inside == Path::new("site.toml");
                restored.database |= inside == Path::new("data/site.db");
            }
            _ => return Err(outside()),
        }
    }
    if !settings {
        return Err(restore_message::not_a_backup(archive));
    }
    move_into(&temporary, folder).map_err(written)?;
    Ok(restored)
}

/// The first folder of `path` and the rest of it, or `None` when any part
/// is not a plain name, such as `..` or a leading `/`.
fn split(path: &Path) -> Option<(OsString, PathBuf)> {
    let mut names = path.components().map(|component| match component {
        Component::Normal(name) => Some(name),
        _ => None,
    });
    let top = names.next()??.to_os_string();
    let rest = names.collect::<Option<PathBuf>>()?;
    Some((top, rest))
}

/// `inside`, checked by [`split`], within the temporary folder.
fn under(temporary: &TempDir, inside: &Path) -> PathBuf {
    let mut target = temporary.path().to_path_buf();
    target.push(inside);
    target
}

/// Moves the unpacked site to `folder`, after taking away `folder` when it
/// is an empty folder. A folder that holds anything stays and fails the move.
fn move_into(temporary: &TempDir, folder: &Path) -> io::Result<()> {
    if folder.is_dir() {
        fs::remove_dir(folder)?;
    }
    fs::rename(temporary.path(), folder)
}

/// Says where the site is and how to start it.
fn success(output: Output, folder: &Path, restored: &Restored) -> Exit {
    let shown = folder.display().to_string();
    match output {
        Output::Human => {
            let database = if restored.database {
                "the database is back as it was at the backup"
            } else {
                "the backup held no database, so Khalid makes a new one at the first start"
            };
            let said = format!(
                "Restored {} files into {shown}, and {database}.\nStart it with `khalid --site {shown} serve`.\n",
                restored.files
            );
            report::text(&said, Exit::Success)
        }
        Output::Json => report::json(
            &RestoreBody {
                folder: shown,
                files: restored.files,
                database: restored.database,
            },
            Exit::Success,
        ),
    }
}
