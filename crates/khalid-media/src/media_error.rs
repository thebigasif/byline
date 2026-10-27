//! What can go wrong while making or keeping derived images.

use std::io;
use std::path::PathBuf;

use khalid_core::SitePathError;

use crate::Format;

/// Why an image could not be derived or cached.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum MediaError {
    /// The cache folder could not be used inside the site folder.
    #[error("the media cache folder could not be used")]
    CachePath(#[from] SitePathError),
    /// A cached file exists but could not be read.
    #[error("could not read {}", path.display())]
    Read {
        /// The file.
        path: PathBuf,
        /// What the file system reported.
        #[source]
        source: io::Error,
    },
    /// The file is not a JPEG, PNG or WebP image, judged by its bytes.
    #[error("the file is not a JPEG, PNG or WebP image")]
    UnsupportedImage,
    /// The file claims to be an image but could not be decoded.
    #[error("the image could not be decoded")]
    Decode {
        /// What the decoder reported.
        #[source]
        source: image::ImageError,
    },
    /// The pixels could not be wrapped for resizing.
    #[error("the image could not be prepared for resizing")]
    ResizeBuffer(#[from] fast_image_resize::ImageBufferError),
    /// Resizing failed.
    #[error("the image could not be resized")]
    Resize(#[from] fast_image_resize::ResizeError),
    /// An encoder from the `image` crate failed.
    #[error("the image could not be encoded as {}", format.extension())]
    Encode {
        /// The format being written.
        format: Format,
        /// What the encoder reported.
        #[source]
        source: image::ImageError,
    },
    /// The WebP encoder failed. Its error type carries only a code.
    #[error("the image could not be encoded as WebP ({code})")]
    EncodeWebp {
        /// The encoder's error code, such as `VP8_ENC_ERROR_OUT_OF_MEMORY`.
        code: String,
    },
    /// The blocking thread that makes images stopped before it finished.
    #[error("the image worker stopped before it finished")]
    Worker {
        /// What the runtime reported.
        #[source]
        source: tokio::task::JoinError,
    },
    /// A request to a bucket could not be signed.
    #[error("the request to the bucket could not be signed")]
    Sign {
        /// What the MAC reported.
        #[source]
        source: hmac::digest::InvalidLength,
    },
    /// A derived file could not be written.
    #[error("could not write {}", path.display())]
    Write {
        /// The file.
        path: PathBuf,
        /// What the file system reported.
        #[source]
        source: io::Error,
    },
}
