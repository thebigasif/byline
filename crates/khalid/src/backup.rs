//! `khalid backup`: one `.tar.gz` archive of the site folder (plan section
//! 4.1). The database goes in as a whole copy made while the server may
//! still run, and what Khalid makes again, the built site in `public/` and
//! the image sizes in `data/media/`, stays out.

use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use flate2::Compression;
use flate2::write::GzEncoder;
use khalid_core::{Clock, UserMessage};
use khalid_db::Database;
use serde::Serialize;
use tempfile::{NamedTempFile, TempDir};

use crate::cli::Output;
use crate::exit::Exit;
use crate::{backup_message, db, report};

/// The paths inside a site, from its root, that a backup leaves out: the
/// built site, image sizes, and the live database files, whose whole copy
/// goes in instead.
const LEFT_OUT: [&str; 6] = [
    "public",
    "data/media",
    "data/site.db",
    "data/site.db-wal",
    "data/site.db-shm",
    "data/site.db-journal",
];

/// The JSON shape of `khalid backup --json`.
#[derive(Serialize)]
struct BackupBody {
    archive: String,
    files: usize,
    bytes: u64,
    database: bool,
}

/// What went into an archive.
struct Written {
    files: usize,
    database: bool,
}

/// Writes one archive of the `site` folder to `out`, or to a file named by
/// the folder and `clock`'s time in the current folder.
pub(crate) fn run(site: &Path, out: Option<&Path>, clock: &dyn Clock, output: Output) -> Exit {
    if !within(site, "site.toml").is_file() {
        return report::failure(output, &backup_message::not_a_site(site), Exit::Failure);
    }
    let folder = fs::canonicalize(site)
        .ok()
        .and_then(|path| Some(path.file_name()?.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "site".to_owned());
    let archive = out.map_or_else(
        || {
            let stamp = clock.now().format("%Y-%m-%d-%H%M%S");
            PathBuf::from(format!("{folder}-backup-{stamp}.tar.gz"))
        },
        Path::to_path_buf,
    );
    if archive.exists() {
        return report::failure(output, &backup_message::exists(&archive), Exit::Failure);
    }
    match write(site, &folder, &archive) {
        Ok(written) => success(output, &archive, &written),
        Err(message) => report::failure(output, &message, Exit::Failure),
    }
}

/// Writes the archive to a temporary file beside `archive`, then moves it
/// into place, never over another file, so a backup is whole or absent.
/// Every entry sits in a folder named `folder`.
fn write(site: &Path, folder: &str, archive: &Path) -> Result<Written, UserMessage> {
    let failed = |error: io::Error| backup_message::write(archive, &error);
    let beside = match archive.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => PathBuf::from("."),
    };
    let temporary = NamedTempFile::new_in(&beside).map_err(failed)?;
    // The archive may be written inside the site; it must not hold itself.
    let skip = fs::canonicalize(temporary.path()).map_err(failed)?;
    let files = site_files(site, &skip).map_err(failed)?;
    let copy = database_copy(site)?;
    let gzip = GzEncoder::new(temporary.as_file(), Compression::default());
    let mut tar = tar::Builder::new(gzip);
    for (path, inside) in &files {
        tar.append_path_with_name(path, format!("{folder}/{inside}"))
            .map_err(failed)?;
    }
    if let Some((_held, file)) = &copy {
        tar.append_path_with_name(file, format!("{folder}/data/site.db"))
            .map_err(failed)?;
    }
    tar.into_inner()
        .and_then(GzEncoder::finish)
        .and_then(File::sync_all)
        .map_err(failed)?;
    temporary
        .persist_noclobber(archive)
        .map_err(|error| failed(error.error))?;
    Ok(Written {
        files: files.len() + usize::from(copy.is_some()),
        database: copy.is_some(),
    })
}

/// Every file in `site` a backup keeps, with its path from the site's root,
/// in name order. Links are left out, and so is the file at `skip`.
fn site_files(site: &Path, skip: &Path) -> io::Result<Vec<(PathBuf, String)>> {
    let mut files = Vec::new();
    let mut folders = vec![(site.to_path_buf(), String::new())];
    while let Some((folder, relative)) = folders.pop() {
        for entry in fs::read_dir(&folder)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let inside = if relative.is_empty() {
                name
            } else {
                format!("{relative}/{name}")
            };
            let kind = entry.file_type()?;
            if LEFT_OUT.contains(&inside.as_str()) || kind.is_symlink() {
                continue;
            }
            if kind.is_dir() {
                folders.push((entry.path(), inside));
            } else if fs::canonicalize(entry.path())? != skip {
                files.push((entry.path(), inside));
            }
        }
    }
    files.sort_by(|a, b| a.1.cmp(&b.1));
    Ok(files)
}

/// A whole copy of the site's database in a temporary folder, held by the
/// returned [`TempDir`], or `None` when the site has no database yet.
fn database_copy(site: &Path) -> Result<Option<(TempDir, PathBuf)>, UserMessage> {
    let mut live = within(site, db::DATA);
    live.push(db::FILE);
    if !live.is_file() {
        return Ok(None);
    }
    let held = tempfile::tempdir().map_err(|error| backup_message::write(&live, &error))?;
    let copy = within(held.path(), db::FILE);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| backup_message::internal())?;
    runtime
        .block_on(async {
            let database = Database::open(&live).await?;
            let copied = database.copy_to(&copy).await;
            database.close().await;
            copied
        })
        .map_err(|error| backup_message::database(&live, &error))?;
    Ok(Some((held, copy)))
}

/// Says where the archive is, how many files it holds and how big it is.
fn success(output: Output, archive: &Path, written: &Written) -> Exit {
    let shown = archive.display().to_string();
    let bytes = fs::metadata(archive).map_or(0, |meta| meta.len());
    match output {
        Output::Human => {
            let database = if written.database {
                "It holds a whole copy of the database."
            } else {
                "The site has no database yet, so there was none to copy."
            };
            let said = format!(
                "Backed up {} files to {shown}, {bytes} bytes. {database}\nTo restore, run `khalid restore {shown} <new-folder>`.\n",
                written.files
            );
            report::text(&said, Exit::Success)
        }
        Output::Json => report::json(
            &BackupBody {
                archive: shown,
                files: written.files,
                bytes,
                database: written.database,
            },
            Exit::Success,
        ),
    }
}

/// `name` inside `folder`. Every `name` here is a fixed one.
fn within(folder: &Path, name: &str) -> PathBuf {
    let mut path = folder.to_path_buf();
    path.push(name);
    path
}
