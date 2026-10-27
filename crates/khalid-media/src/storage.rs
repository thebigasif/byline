//! Where the media library keeps files' bytes: the site's own `media/`
//! folder, or a bucket.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use std::sync::Arc;

use khalid_core::{Clock, S3Keys, S3Settings, SitePath};
use khalid_outbound::Outbound;

use crate::s3_storage::S3Storage;
use crate::whole_file::write_whole;
use crate::{ContentKey, FileName, MediaError, MediaType};

/// The folder in a site that holds the library.
pub(crate) const LIBRARY_FOLDER: &str = "media";

/// Where readers fetch a stored file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FileAddress {
    /// A path the site itself serves, relative to where the blog is
    /// mounted, such as `media/3f2a9c1e0b57d0aa/choosing-a-crm.jpg`.
    OnSite(String),
    /// A full address on another server, such as a bucket's public one.
    Elsewhere(String),
}

/// Where the library keeps files' bytes. Every file is stored as
/// `<key>/<name>`, so two files never share a place and a stored file never
/// changes (ADR "the media library keeps each file with its details").
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Storage {
    /// In the site's `media/` folder, beside each file's `item.toml`.
    Local(LocalStorage),
    /// In an S3-compatible bucket, with only `item.toml` in `media/`.
    S3(Arc<S3Storage>),
}

impl Storage {
    /// Files in the `media/` folder of the site at `site_root`.
    pub fn local(site_root: &Path) -> Self {
        Self::Local(LocalStorage {
            site_root: site_root.to_path_buf(),
        })
    }

    /// Files in the bucket `settings` names, signed with `keys` at the time
    /// `clock` gives and sent through `outbound`, so the bucket's address
    /// must be public.
    pub fn s3(
        settings: &S3Settings,
        keys: S3Keys,
        outbound: Outbound,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self::S3(Arc::new(S3Storage::new(settings, keys, outbound, clock)))
    }

    /// Stores `bytes`, a file of `media_type`, as the file `name` of the item
    /// `key`. Storing the same file again leaves one copy.
    ///
    /// # Errors
    ///
    /// Returns [`MediaError::Write`] when a local file cannot be written,
    /// [`MediaError::Worker`] when the thread that writes it stops, and
    /// [`MediaError::Reach`] or [`MediaError::Bucket`] when a bucket cannot
    /// be reached or refuses the file.
    pub async fn put(
        &self,
        key: &ContentKey,
        name: &FileName,
        media_type: MediaType,
        bytes: Vec<u8>,
    ) -> Result<(), MediaError> {
        match self {
            Self::S3(bucket) => bucket.put(key, name, bytes, media_type).await,
            Self::Local(local) => {
                let path = local.path(key, name)?;
                blocking(move || match write_whole(&path, &bytes) {
                    Ok(()) => Ok(()),
                    Err(source) => Err(MediaError::Write { path, source }),
                })
                .await
            }
        }
    }

    /// The bytes of the file `name` of the item `key`, or `None` when it was
    /// never stored.
    ///
    /// # Errors
    ///
    /// Returns [`MediaError::Read`] when a local file exists but cannot be
    /// read, [`MediaError::Worker`] when the thread that reads it stops, and
    /// [`MediaError::Reach`] or [`MediaError::Bucket`] when a bucket cannot
    /// be reached or refuses.
    pub async fn get(
        &self,
        key: &ContentKey,
        name: &FileName,
    ) -> Result<Option<Vec<u8>>, MediaError> {
        match self {
            Self::S3(bucket) => bucket.get(key, name).await,
            Self::Local(local) => {
                let path = local.path(key, name)?;
                blocking(move || match fs::read(&path) {
                    Ok(bytes) => Ok(Some(bytes)),
                    Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
                    Err(source) => Err(MediaError::Read { path, source }),
                })
                .await
            }
        }
    }

    /// Where readers fetch the file `name` of the item `key`.
    pub fn address(&self, key: &ContentKey, name: &FileName) -> FileAddress {
        match self {
            Self::Local(_) => FileAddress::OnSite(stored_as(LIBRARY_FOLDER, key, name)),
            Self::S3(bucket) => FileAddress::Elsewhere(bucket.address(key, name)),
        }
    }
}

/// Files kept in a site's `media/` folder.
#[derive(Debug, Clone)]
pub struct LocalStorage {
    site_root: PathBuf,
}

impl LocalStorage {
    /// The path of the file `name` of the item `key`, which cannot lead
    /// outside the site.
    fn path(&self, key: &ContentKey, name: &FileName) -> Result<PathBuf, MediaError> {
        let relative = stored_as(LIBRARY_FOLDER, key, name);
        Ok(SitePath::within(&self.site_root, &relative)?.resolve()?)
    }
}

/// `<prefix>/<key>/<name>`, or `<key>/<name>` when `prefix` is empty.
pub(crate) fn stored_as(prefix: &str, key: &ContentKey, name: &FileName) -> String {
    let prefix = prefix.trim_end_matches('/');
    let name = name.as_str();
    if prefix.is_empty() {
        format!("{key}/{name}")
    } else {
        format!("{prefix}/{key}/{name}")
    }
}

/// Runs file work on a blocking thread (RULES.md BL-RUN-7).
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, MediaError> + Send + 'static,
) -> Result<T, MediaError> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|source| MediaError::Worker { source })?
}
