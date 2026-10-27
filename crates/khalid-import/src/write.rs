//! Writing a plan's files into a site folder, whatever the source
//! (RULES.md BL-RUN-9, BL-SEC-4).

use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

use khalid_core::{SitePath, SitePathError};

/// One file an import plans to write.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PlannedFile {
    /// The path from the site folder, such as `content/en/posts/...`.
    pub path: PathBuf,
    /// The whole file.
    pub text: String,
}

/// Where an imported item's file goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Place {
    /// A published post, under `content/<lang>/posts/`.
    Post,
    /// A post not yet published, under `drafts/<lang>/` (ADR 0030).
    Draft,
    /// A page, under `content/<lang>/pages/`.
    Page,
}

/// Why a plan could not be written.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum WriteError {
    /// Files the plan would write already exist, so nothing was written.
    #[error("{} files the import would write already exist", paths.len())]
    Exists {
        /// The files that exist, from the site folder.
        paths: Vec<PathBuf>,
    },
    /// A planned path is not inside the site folder, for example because a
    /// folder on the way is a link to somewhere else.
    #[error("{} is not inside the site folder", path.display())]
    Path {
        /// The planned path, from the site folder.
        path: PathBuf,
        /// What the check found.
        #[source]
        source: SitePathError,
    },
    /// A file could not be written. The files written before it stay.
    #[error("{} could not be written", path.display())]
    Write {
        /// The file, from the site folder.
        path: PathBuf,
        /// The files written before it, from the site folder.
        written: Vec<PathBuf>,
        /// What the system said.
        #[source]
        source: io::Error,
    },
}

/// Writes every one of `files` into the site folder at `site`, each one
/// atomically: to a temporary file in the same folder, synced, then
/// renamed. Returns the files written, from the site folder.
///
/// # Errors
///
/// Returns [`WriteError::Exists`] when any planned file exists already,
/// before anything is written; [`WriteError::Path`] when a planned path
/// leads outside the site folder, also before anything is written; and
/// [`WriteError::Write`] when a file cannot be written.
pub(crate) fn write_files(files: &[PlannedFile], site: &Path) -> Result<Vec<PathBuf>, WriteError> {
    let mut targets = Vec::with_capacity(files.len());
    for file in files {
        let path = &file.path;
        let failed = |source| WriteError::Path {
            path: path.clone(),
            source,
        };
        let within = SitePath::within(site, &path.to_string_lossy()).map_err(failed)?;
        targets.push((within.resolve().map_err(failed)?, file));
    }
    let exists: Vec<PathBuf> = targets
        .iter()
        .filter(|(target, _)| target.exists())
        .map(|(_, file)| file.path.clone())
        .collect();
    if !exists.is_empty() {
        return Err(WriteError::Exists { paths: exists });
    }
    let mut written = Vec::with_capacity(targets.len());
    for (target, file) in targets {
        if let Err(source) = atomic_write(&target, file.text.as_bytes()) {
            let path = file.path.clone();
            return Err(WriteError::Write {
                path,
                written,
                source,
            });
        }
        written.push(file.path.clone());
    }
    Ok(written)
}

/// Writes `bytes` to `target` through a temporary file beside it, so a
/// reader sees the whole file or none of it.
fn atomic_write(target: &Path, bytes: &[u8]) -> io::Result<()> {
    let folder = target.parent().ok_or(io::ErrorKind::InvalidInput)?;
    fs::create_dir_all(folder)?;
    let name = target.file_name().ok_or(io::ErrorKind::InvalidInput)?;
    let mut partial = target.to_path_buf();
    partial.set_file_name(format!(
        ".{}.{}.partial",
        name.to_string_lossy(),
        std::process::id()
    ));
    let result = (|| {
        let mut file = fs::File::create(&partial)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&partial, target)
    })();
    if result.is_err() {
        // The partial file is ours, and of no use once the write failed.
        let _ = fs::remove_file(&partial);
    }
    result
}
