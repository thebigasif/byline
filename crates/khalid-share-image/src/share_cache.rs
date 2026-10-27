//! Where drawn cards live: `data/share/<key>.png` in the site.

use std::fs;
use std::io;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use khalid_core::SitePath;

use crate::{ShareCard, ShareImageError, ShareKey, render};

/// The folder for drawn cards, inside the site's `data/` folder, never
/// under `content/` (RULES.md BL-FS-2).
const SHARE_FOLDER: &str = "data/share";

/// The drawn cards of one site, kept on disk by content key (ADR 0016).
#[derive(Debug, Clone)]
pub struct ShareCache {
    site_root: PathBuf,
}

/// One card drawn, or read back whole from the cache.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Drawn {
    /// The card's content key, the name of its file.
    pub key: ShareKey,
    /// The PNG bytes.
    pub png: Vec<u8>,
    /// Whether the bytes were read from the cache rather than drawn.
    pub from_cache: bool,
}

impl ShareCache {
    /// The cache inside `site_root/data/share/`. Nothing is read or made yet.
    pub fn new(site_root: &Path) -> Self {
        Self {
            site_root: site_root.to_path_buf(),
        }
    }

    /// Where the card with `key` is kept.
    ///
    /// # Errors
    ///
    /// Returns [`ShareImageError::CachePath`] when the folder leads outside
    /// the site through a symlink.
    pub fn path(&self, key: &ShareKey) -> Result<PathBuf, ShareImageError> {
        let relative = format!("{SHARE_FOLDER}/{}.png", key.as_str());
        Ok(SitePath::within(&self.site_root, &relative)?.resolve()?)
    }

    /// The card with `key` as PNG bytes, or `None` when it was never drawn.
    ///
    /// # Errors
    ///
    /// Returns [`ShareImageError::Read`] when the file exists but cannot be
    /// read, and [`ShareImageError::CachePath`] as [`ShareCache::path`] does.
    pub fn read(&self, key: &ShareKey) -> Result<Option<Vec<u8>>, ShareImageError> {
        let path = self.path(key)?;
        match fs::read(&path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(ShareImageError::Read { path, source }),
        }
    }

    /// Draws `card` and keeps it, or returns the drawing the cache already
    /// holds, which is byte for byte the same. Drawing takes some CPU, so
    /// async callers run this in `spawn_blocking` (RULES.md BL-RUN-7).
    ///
    /// # Errors
    ///
    /// Returns the errors of [`render`] on a first draw, and those of
    /// [`ShareCache::read`] and [`ShareImageError::Write`] on any draw.
    pub fn draw(&self, card: &ShareCard) -> Result<Drawn, ShareImageError> {
        let key = ShareKey::of(card);
        if let Some(png) = self.read(&key)? {
            return Ok(Drawn {
                key,
                png,
                from_cache: true,
            });
        }
        let png = render(card)?;
        self.write(&key, &png)?;
        Ok(Drawn {
            key,
            png,
            from_cache: false,
        })
    }

    /// Keeps `png` as the card with `key`. The file appears whole or not at
    /// all: it is written beside its final name, flushed to disk and then
    /// renamed (RULES.md BL-RUN-9).
    ///
    /// # Errors
    ///
    /// Returns [`ShareImageError::Write`] when the folder or the file
    /// cannot be written.
    pub fn write(&self, key: &ShareKey, png: &[u8]) -> Result<(), ShareImageError> {
        let path = self.path(key)?;
        if let Err(source) = write_whole(&path, png) {
            return Err(ShareImageError::Write { path, source });
        }
        Ok(())
    }
}

/// Writes `bytes` to `path`, making its folder first. The bytes go to a
/// partial file beside it, are flushed to disk, and the partial file is
/// renamed over `path`, so a reader never sees half a file.
fn write_whole(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let folder = path.parent().ok_or(io::ErrorKind::InvalidInput)?;
    let name = path.file_name().ok_or(io::ErrorKind::InvalidInput)?;
    fs::create_dir_all(folder)?;
    let mut partial = path.to_path_buf();
    partial.set_file_name(format!(
        ".{}.{}.partial",
        name.to_string_lossy(),
        std::process::id()
    ));
    let mut file = fs::File::create(&partial)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    fs::rename(&partial, path)
}
