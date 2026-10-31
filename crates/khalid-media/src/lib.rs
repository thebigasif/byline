//! The media library, and images for the public site made once at publish.
//!
//! This crate owns two things. The library: files uploaded through the
//! Admin API, checked by their bytes, named by a hash of them, and kept
//! with their details (ADR "the media library keeps each file with its
//! details"). And derived images: one original turned into the sizes and
//! formats a page offers, WebP in a responsive set of widths plus the
//! original's format for older browsers. AVIF joins when its encoder passes
//! the dependency policy. Every derived file is named by a hash of the
//! original's bytes and kept in the site's `data/media/` folder, so a file
//! that did not change is never encoded twice (survey lesson 5, RULES.md
//! BL-PERF-5).
//!
//! It must not know about HTTP routes, templates or the content tree. The
//! render crate shows what this crate made; the wiring code connects them.

mod content_key;
mod crop;
mod derive;
mod file_name;
mod focal_point;
mod folder;
mod library;
mod media_cache;
mod media_edit;
mod media_error;
mod media_item;
mod media_type;
mod original;
mod s3_storage;
mod sig_v4;
mod stock;
mod stock_unsplash;
mod storage;
mod variant;
mod whole_file;

pub use content_key::ContentKey;
pub use crop::Crop;
pub use derive::{DerivedFile, DerivedImage, derive, derive_in_background};
pub use file_name::{FileName, ParseFileNameError};
pub use focal_point::{FocalPoint, ParseFocalPointError};
pub use folder::{Folder, ParseFolderError};
pub use library::{Library, Upload, UploadMediaError, Uploaded};
pub use media_cache::MediaCache;
pub use media_edit::{EditMediaError, MediaEdit};
pub use media_error::MediaError;
pub use media_item::{MediaItem, ParseItemError, Pixels};
pub use media_type::{Kind, MediaType, ParseMediaTypeError, Refusal};
pub use original::Original;
pub use s3_storage::S3Storage;
pub use sig_v4::{SigV4, Signed};
pub use stock::{Access, FoundPicture, PictureSource, StockApis, StockError, StockSearch};
pub use storage::{FileAddress, LIBRARY_ADDRESS, LocalStorage, Storage};
pub use variant::{Format, Variant, widths};
