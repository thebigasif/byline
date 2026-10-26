//! Writes a reader never sees half done (RULES.md BL-RUN-9).

use std::io::{self, Write};
use std::path::Path;

use tempfile::NamedTempFile;

/// Writes `text` to a new file `name` in `folder`: first to a temporary file
/// in the same folder, then flushed to disk, then renamed into place.
///
/// An existing file is never replaced; the write fails with
/// [`io::ErrorKind::AlreadyExists`] and leaves it as it was.
pub(crate) fn create(folder: &Path, name: &str, text: &str) -> io::Result<()> {
    let mut file = NamedTempFile::new_in(folder)?;
    file.write_all(text.as_bytes())?;
    file.as_file().sync_all()?;
    let mut path = folder.to_path_buf();
    path.push(name);
    file.persist_noclobber(&path).map_err(|error| error.error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn writes_the_file_and_leaves_no_temporary_file() {
        let temp = tempfile::tempdir().unwrap();
        create(temp.path(), "site.toml", "name = \"x\"\n").unwrap();
        let names: Vec<_> = fs::read_dir(temp.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(names, ["site.toml"]);
    }

    #[test]
    fn never_replaces_a_file() {
        let temp = tempfile::tempdir().unwrap();
        create(temp.path(), "post.md", "first").unwrap();
        let error = create(temp.path(), "post.md", "second").unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        let mut path = temp.path().to_path_buf();
        path.push("post.md");
        assert_eq!(fs::read_to_string(path).unwrap(), "first");
    }
}
