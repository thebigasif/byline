//! The media library: every uploaded file with its details.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, PoisonError, RwLock};

use khalid_core::{Clock, Locale, SitePath};

use crate::storage::LIBRARY_FOLDER;
use crate::whole_file::write_whole;
use crate::{
    ContentKey, FileName, FocalPoint, Folder, Kind, MediaError, MediaItem, MediaType, Original,
    Pixels, Refusal, Storage,
};

/// The file beside each item that holds its details.
const ITEM_FILE: &str = "item.toml";

/// The media library of one site. It reads every `media/<key>/item.toml`
/// once when it opens, keeps the details in memory, and writes each change
/// back to its file (ADR "the media library keeps each file with its
/// details").
#[derive(Debug)]
pub struct Library {
    site_root: PathBuf,
    storage: Storage,
    locale: Locale,
    clock: Arc<dyn Clock>,
    items: RwLock<BTreeMap<ContentKey, MediaItem>>,
}

/// A file someone sends to the library.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Upload {
    /// The name it was sent with, such as `Choosing a CRM.jpg`.
    pub name: String,
    /// The folder to file it in.
    pub folder: Folder,
    /// Its bytes.
    pub bytes: Vec<u8>,
    /// Who sends it: a staff member or an integration key's name.
    pub by: String,
}

impl Upload {
    /// `bytes`, sent as `name` by `by`, to file in `folder`.
    pub fn new(name: &str, folder: Folder, bytes: Vec<u8>, by: &str) -> Self {
        Self {
            name: name.to_owned(),
            folder,
            bytes,
            by: by.to_owned(),
        }
    }
}

/// What an upload did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Uploaded {
    /// The file is new and now in the library.
    New(MediaItem),
    /// The library already has these bytes, as this item, which is
    /// unchanged.
    Duplicate(MediaItem),
}

/// Why an upload was not added.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum UploadMediaError {
    /// The file is not one the library keeps.
    #[error(transparent)]
    Refused(Refusal),
    /// The file could not be stored.
    #[error(transparent)]
    Media(#[from] MediaError),
}

impl Library {
    /// Opens the library of the site at `site_root`, whose files are in
    /// `storage`. Uploaded names are made into slugs with `locale`'s
    /// spelling, and times come from `clock`.
    ///
    /// # Errors
    ///
    /// Returns [`MediaError::Read`] when the `media/` folder or an
    /// `item.toml` cannot be read, [`MediaError::Item`] when an `item.toml`
    /// is not valid, and [`MediaError::Worker`] when the reading thread
    /// stops.
    pub async fn open(
        site_root: &Path,
        storage: Storage,
        locale: &Locale,
        clock: Arc<dyn Clock>,
    ) -> Result<Self, MediaError> {
        let folder = SitePath::within(site_root, LIBRARY_FOLDER)?.resolve()?;
        let items = tokio::task::spawn_blocking(move || read_items(&folder))
            .await
            .map_err(|source| MediaError::Worker { source })??;
        Ok(Self {
            site_root: site_root.to_path_buf(),
            storage,
            locale: locale.clone(),
            clock,
            items: RwLock::new(items),
        })
    }

    /// Adds `upload`, or finds the item that already has its bytes. The type
    /// is read from the bytes; an image is decoded, turned upright and
    /// encoded again in its own format before it is stored (RULES.md
    /// BL-SEC-6).
    ///
    /// # Errors
    ///
    /// Returns [`UploadMediaError::Refused`] for a type the library does not
    /// keep, an image that cannot be read, or a file over its kind's limit,
    /// and [`UploadMediaError::Media`] when it cannot be stored.
    pub async fn upload(&self, upload: Upload) -> Result<Uploaded, UploadMediaError> {
        let id = ContentKey::of(&upload.bytes);
        if let Some(existing) = self.get(&id) {
            return Ok(Uploaded::Duplicate(existing));
        }
        let media_type = MediaType::sniff(&upload.bytes).map_err(UploadMediaError::Refused)?;
        let kind = media_type.kind();
        let size = u64::try_from(upload.bytes.len()).unwrap_or(u64::MAX);
        kind.check_size(size).map_err(UploadMediaError::Refused)?;
        let (stored, pixels) = if kind == Kind::Image {
            let (bytes, pixels) = cleaned(upload.bytes).await?;
            (bytes, Some(pixels))
        } else {
            (upload.bytes, None)
        };
        let name = FileName::new(&upload.name, media_type, &self.locale);
        let now = self.clock.now();
        let item = MediaItem {
            id,
            name,
            folder: upload.folder,
            media_type,
            bytes: u64::try_from(stored.len()).unwrap_or(u64::MAX),
            pixels,
            alt: String::new(),
            caption: String::new(),
            focal_point: FocalPoint::default(),
            uploaded_at: now,
            uploaded_by: upload.by.clone(),
            updated_at: now,
            updated_by: upload.by,
        };
        self.storage
            .put(&item.id, &item.name, media_type, stored)
            .await?;
        self.save(&item).await?;
        Ok(Uploaded::New(item))
    }

    /// The item `id`, if the library has it.
    pub fn get(&self, id: &ContentKey) -> Option<MediaItem> {
        let items = self.items.read().unwrap_or_else(PoisonError::into_inner);
        items.get(id).cloned()
    }

    /// Every item, newest upload first.
    pub fn items(&self) -> Vec<MediaItem> {
        let items = self.items.read().unwrap_or_else(PoisonError::into_inner);
        let mut all: Vec<MediaItem> = items.values().cloned().collect();
        all.sort_by(|a, b| b.uploaded_at.cmp(&a.uploaded_at).then(a.id.cmp(&b.id)));
        all
    }

    /// Writes `item`'s `item.toml`, then keeps it in memory.
    async fn save(&self, item: &MediaItem) -> Result<(), MediaError> {
        let relative = format!("{LIBRARY_FOLDER}/{}/{ITEM_FILE}", item.id);
        let path = SitePath::within(&self.site_root, &relative)?.resolve()?;
        let text = item.to_toml();
        tokio::task::spawn_blocking(move || match write_whole(&path, text.as_bytes()) {
            Ok(()) => Ok(()),
            Err(source) => Err(MediaError::Write { path, source }),
        })
        .await
        .map_err(|source| MediaError::Worker { source })??;
        let mut items = self.items.write().unwrap_or_else(PoisonError::into_inner);
        items.insert(item.id.clone(), item.clone());
        Ok(())
    }
}

/// An image's bytes encoded again, upright and without hidden data, and its
/// size, made on a blocking thread (RULES.md BL-RUN-7).
async fn cleaned(bytes: Vec<u8>) -> Result<(Vec<u8>, Pixels), UploadMediaError> {
    let work = tokio::task::spawn_blocking(move || {
        let original = Original::decode(&bytes).map_err(|_| Refusal::Broken)?;
        let pixels = Pixels {
            width: original.width(),
            height: original.height(),
        };
        Ok::<_, Refusal>(original.reencode().map(|bytes| (bytes, pixels)))
    });
    let done = work.await.map_err(|source| MediaError::Worker { source })?;
    Ok(done.map_err(UploadMediaError::Refused)??)
}

/// Every item in the library `folder`, which may not exist yet. A folder
/// whose name is not a key, or that has no `item.toml`, is not an item.
fn read_items(folder: &Path) -> Result<BTreeMap<ContentKey, MediaItem>, MediaError> {
    let failed = |source| MediaError::Read {
        path: folder.to_path_buf(),
        source,
    };
    let entries = match fs::read_dir(folder) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(source) => return Err(failed(source)),
    };
    let mut items = BTreeMap::new();
    for entry in entries {
        let entry = entry.map_err(failed)?;
        let Some(id) = entry.file_name().to_str().and_then(ContentKey::parse) else {
            continue;
        };
        let mut path = entry.path();
        path.push(ITEM_FILE);
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(source) => return Err(MediaError::Read { path, source }),
        };
        let item = MediaItem::from_toml(id.clone(), &text)
            .map_err(|source| MediaError::Item { path, source })?;
        items.insert(id, item);
    }
    Ok(items)
}
