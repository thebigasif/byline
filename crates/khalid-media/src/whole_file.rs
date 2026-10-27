//! Writing a file so it appears whole or not at all.

use std::fs;
use std::io::{self, Write as _};
use std::path::Path;

/// Writes `bytes` to `path`, making its folder first. The bytes go to a
/// partial file beside it, are flushed to disk, and the partial file is
/// renamed over `path` (RULES.md BL-RUN-9), so a reader never sees half a
/// file and a crash leaves the old one.
pub(crate) fn write_whole(path: &Path, bytes: &[u8]) -> io::Result<()> {
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
