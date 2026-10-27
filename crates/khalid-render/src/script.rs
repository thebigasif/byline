//! The public site's one script: polish on top of pages that read
//! completely without it.

use serde::Serialize;

use crate::{Asset, RenderError};

/// What the script does, in plain JavaScript with no dependencies: copy a
/// heading's address, open an enlarged image in a dialog, open search with
/// `/` or Ctrl+K, and mark the section being read.
const READER_JS: &str = include_str!("../assets/reader.js");

/// Builds the reader script (plan section 4.12) as a hashed asset with its
/// compressed copies. Pages load it deferred, so it never holds up reading.
///
/// # Errors
///
/// Returns [`RenderError::Compress`] if a compressor fails.
pub fn reader_script() -> Result<Asset, RenderError> {
    Asset::build(
        "reader",
        "js",
        "text/javascript; charset=utf-8",
        READER_JS.as_bytes().to_vec(),
    )
}

/// The `script` tag every page carries when the site serves the reader
/// script.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ScriptTag {
    /// The script's address, such as `/blog/assets/reader.3f2a9c1e.js`.
    pub src: String,
    /// Its [`Asset::integrity`], so the browser runs only these bytes.
    pub integrity: String,
}

impl ScriptTag {
    /// The tag for `script`, served from the folder `assets`, such as
    /// `/blog/assets/`.
    pub fn new(script: &Asset, assets: &str) -> Self {
        Self {
            src: format!("{assets}{}", script.name()),
            integrity: script.integrity(),
        }
    }
}
