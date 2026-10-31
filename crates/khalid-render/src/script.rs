//! The public site's one script: polish on top of pages that read
//! completely without it.

use serde::Serialize;

use crate::asset::{hashed_name, integrity_of};
use crate::{Asset, RenderError};

/// What the script does, in plain JavaScript with no dependencies: copy a
/// heading's address, open an enlarged image in a dialog, open search with
/// `/` or Ctrl+K, and mark the section being read.
const READER_JS: &str = include_str!("../assets/reader.js");

/// The script's name before its hash, and its extension.
const STEM: &str = "reader";
const EXTENSION: &str = "js";

/// Builds the reader script (plan section 4.12) as a hashed asset with its
/// compressed copies. Pages load it deferred, so it never holds up reading.
///
/// # Errors
///
/// Returns [`RenderError::Compress`] if a compressor fails.
pub fn reader_script() -> Result<Asset, RenderError> {
    Asset::build(
        STEM,
        EXTENSION,
        "text/javascript; charset=utf-8",
        READER_JS.as_bytes().to_vec(),
    )
}

/// The reader script without its compressed copies: the name and integrity
/// value [`reader_script`] gives it, and its bytes. Making them costs one
/// hash, so a server that compresses as it answers can start at once.
pub fn reader_script_file() -> ScriptFile {
    let body = READER_JS.as_bytes();
    ScriptFile {
        name: hashed_name(STEM, EXTENSION, body),
        integrity: integrity_of(body),
        body,
    }
}

/// A script's hashed name, integrity value and bytes, from
/// [`reader_script_file`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptFile {
    /// The hashed file name, such as `reader.3f2a9c1e.js`.
    pub name: String,
    /// Its SHA-256 as a Subresource Integrity value.
    pub integrity: String,
    /// The file as built.
    pub body: &'static [u8],
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

    /// The tag for `script`, served from the folder `assets`, without
    /// building its compressed copies.
    pub fn of_file(script: &ScriptFile, assets: &str) -> Self {
        Self {
            src: format!("{assets}{}", script.name),
            integrity: script.integrity.clone(),
        }
    }
}
