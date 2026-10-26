//! What can go wrong while reading a generated site into render and lint
//! inputs.

use khalid_content::{BlockError, ContentError};
use khalid_core::{ConfigError, ParseUrlError};

/// A site could not be read into render and lint inputs.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PrepareError {
    /// `site.toml` is missing or wrong.
    #[error("the site settings could not be read")]
    Config(#[from] ConfigError),
    /// A content file is missing or wrong.
    #[error("a content file could not be read")]
    Content(#[from] ContentError),
    /// A post's body holds a broken block.
    #[error("a post's body could not be rendered")]
    Body(#[from] BlockError),
    /// The site address and a post's path do not form a page address.
    #[error("a page address could not be formed")]
    Address(#[from] ParseUrlError),
}
