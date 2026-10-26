//! Where derived files live: `data/media/<key>/<width>.<ext>` in the site.

use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

use khalid_core::SitePath;

use crate::{ContentKey, MediaError, Variant};

/// The folder for derived media, inside the site's `data/` folder, never
/// under `content/` (RULES.md BL-FS-2).
const MEDIA_FOLDER: &str = "data/media";

/// The derived files of one site, kept on disk by content key.
#[derive(Debug, Clone)]
pub struct MediaCache {
    site_root: PathBuf,
}

impl MediaCache {
    /// The cache inside `site_root/data/media/`. Nothing is read or made yet.
    pub fn new(site_root: &Path) -> Self {
        Self {
            site_root: site_root.to_path_buf(),
        }
    }

    /// Where `variant` of the original with `key` is kept.
    ///
    /// # Errors
    ///
    /// Returns [`MediaError::CachePath`] when the folder leads outside the
    /// site through a symlink.
    pub fn path(&self, key: &ContentKey, variant: &Variant) -> Result<PathBuf, MediaError> {
        let relative = format!("{MEDIA_FOLDER}/{key}/{}", variant.file_name());
        Ok(SitePath::within(&self.site_root, &relative)?.resolve()?)
    }

    /// The cached bytes of `variant`, or `None` when it was never made.
    ///
    /// # Errors
    ///
    /// Returns [`MediaError::Read`] when the file exists but cannot be read.
    pub fn read(&self, key: &ContentKey, variant: &Variant) -> Result<Option<Vec<u8>>, MediaError> {
        let path = self.path(key, variant)?;
        match fs::read(&path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(MediaError::Read { path, source }),
        }
    }

    /// Keeps `bytes` as `variant`. The file appears whole or not at all: it
    /// is written beside its final name, flushed to disk and then renamed
    /// (RULES.md BL-RUN-9), so a reader never sees half an image.
    ///
    /// # Errors
    ///
    /// Returns [`MediaError::Write`] when the folder or the file cannot be
    /// written.
    pub fn write(
        &self,
        key: &ContentKey,
        variant: &Variant,
        bytes: &[u8],
    ) -> Result<PathBuf, MediaError> {
        let path = self.path(key, variant)?;
        let failed = |source| MediaError::Write {
            path: path.clone(),
            source,
        };
        let folder = path
            .parent()
            .ok_or_else(|| failed(io::ErrorKind::InvalidInput.into()))?;
        fs::create_dir_all(folder).map_err(failed)?;
        let mut partial = path.clone();
        partial.set_file_name(format!(
            ".{}.{}.partial",
            variant.file_name(),
            std::process::id()
        ));
        let mut file = fs::File::create(&partial).map_err(failed)?;
        file.write_all(bytes).map_err(failed)?;
        file.sync_all().map_err(failed)?;
        fs::rename(&partial, &path).map_err(failed)?;
        Ok(path)
    }
}
