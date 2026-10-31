//! A built file the site serves, with a content-hashed name and its
//! compressed copies made once.

use std::fmt::Write as _;
use std::io::{self, Write as _};
use std::sync::Arc;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use flate2::Compression;
use flate2::write::GzEncoder;
use sha2::{Digest, Sha256};

use crate::RenderError;

/// Brotli's best quality. Assets are compressed once when built, so the
/// slowest, smallest setting costs nothing per request.
const BROTLI_QUALITY: u32 = 11;

/// Brotli's window size, as a power of two: 4 MB, more than any asset needs.
const BROTLI_WINDOW: u32 = 22;

/// How many hex digits of the content hash go into the name. Eight digits
/// make a collision between two versions of one file vanishingly unlikely.
const HASH_DIGITS: usize = 8;

/// One built file, such as the stylesheet, ready to serve.
///
/// The name carries a hash of the content, such as `site.3f2a9c1e.css`, so
/// it can be cached for a year: a change makes a new name. The gzip and
/// Brotli copies are made once here, never per request (BL-PERF-2). The same
/// build serves `khalid serve` and `khalid build`, so what a writer sees
/// while working is what ships (survey lesson 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    name: String,
    content_type: &'static str,
    body: Arc<[u8]>,
    gzip: Arc<[u8]>,
    brotli: Arc<[u8]>,
}

impl Asset {
    /// Builds the asset `stem.<hash>.extension` from `body`, with its gzip
    /// and Brotli copies.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::Compress`] if a compressor fails, which only
    /// happens when memory runs out.
    pub fn build(
        stem: &str,
        extension: &str,
        content_type: &'static str,
        body: Vec<u8>,
    ) -> Result<Self, RenderError> {
        let name = hashed_name(stem, extension, &body);
        let gzip = gzip(&body).map_err(|source| RenderError::Compress { source })?;
        let brotli = brotli(&body).map_err(|source| RenderError::Compress { source })?;
        Ok(Self {
            name,
            content_type,
            body: body.into(),
            gzip: gzip.into(),
            brotli: brotli.into(),
        })
    }

    /// The name [`Asset::build`] gives `body`, such as `site.3f2a9c1e.css`,
    /// without making the compressed copies.
    pub fn hashed_name(stem: &str, extension: &str, body: &[u8]) -> String {
        hashed_name(stem, extension, body)
    }

    /// The hashed file name, such as `site.3f2a9c1e.css`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The media type, such as `text/css; charset=utf-8`.
    pub fn content_type(&self) -> &'static str {
        self.content_type
    }

    /// The file as built.
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// The gzip copy, for clients that accept gzip but not Brotli.
    pub fn gzip(&self) -> &[u8] {
        &self.gzip
    }

    /// The Brotli copy, the smallest.
    pub fn brotli(&self) -> &[u8] {
        &self.brotli
    }

    /// The file's SHA-256 as a Subresource Integrity value, such as
    /// `sha256-47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=`. A tag that
    /// carries it loads only these exact bytes, and a Content Security
    /// Policy can allow the file by the same value.
    pub fn integrity(&self) -> String {
        integrity_of(&self.body)
    }
}

/// `stem.<hash>.extension`, the name [`Asset::build`] gives `body`.
pub(crate) fn hashed_name(stem: &str, extension: &str, body: &[u8]) -> String {
    let hash = Sha256::digest(body);
    let mut name = format!("{stem}.");
    for byte in hash.iter().take(HASH_DIGITS / 2) {
        let _ = write!(name, "{byte:02x}");
    }
    name.push('.');
    name.push_str(extension);
    name
}

/// The SHA-256 of `body` as a Subresource Integrity value.
pub(crate) fn integrity_of(body: &[u8]) -> String {
    format!("sha256-{}", STANDARD.encode(Sha256::digest(body)))
}

fn gzip(body: &[u8]) -> io::Result<Vec<u8>> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(body)?;
    encoder.finish()
}

fn brotli(body: &[u8]) -> io::Result<Vec<u8>> {
    let mut out = Vec::new();
    {
        let mut writer =
            brotli::CompressorWriter::new(&mut out, 4096, BROTLI_QUALITY, BROTLI_WINDOW);
        writer.write_all(body)?;
        writer.flush()?;
    }
    Ok(out)
}
