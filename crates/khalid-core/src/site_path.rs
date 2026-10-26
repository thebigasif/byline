//! Paths that cannot leave the site folder.

use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// A path inside a site folder, built from text nobody vouches for.
///
/// Every path that comes from a request, an upload or a config file becomes a
/// `SitePath` before it touches the disk (RULES.md BL-SEC-4). It is checked
/// twice. [`SitePath::within`] refuses text that could climb out: an absolute
/// path, `.` or `..`, an empty segment, a backslash, a colon or a NUL byte.
/// [`SitePath::resolve`] then follows symlinks on disk at the moment of use, so
/// a symlink that points outside is refused even when it was created after the
/// path was built. Resolve right before opening the file.
///
/// # Examples
///
/// ```
/// use std::path::Path;
///
/// use khalid_core::SitePath;
///
/// let path = SitePath::within(Path::new("/srv/site"), "posts/why-links-matter.md")?;
/// assert_eq!(path.relative(), Path::new("posts/why-links-matter.md"));
/// assert!(SitePath::within(Path::new("/srv/site"), "../secrets.toml").is_err());
/// # Ok::<(), khalid_core::SitePathError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SitePath {
    root: PathBuf,
    relative: PathBuf,
    full: PathBuf,
}

impl SitePath {
    /// Builds the path `untrusted` names inside `root`, which the caller trusts.
    ///
    /// `untrusted` uses `/` between segments, whatever the platform. Percent
    /// signs stay literal: decoding is the caller's job, before this check.
    ///
    /// # Errors
    ///
    /// Returns a [`SitePathError`] naming the first rule `untrusted` breaks.
    pub fn within(root: &Path, untrusted: &str) -> Result<Self, SitePathError> {
        if untrusted.is_empty() {
            return Err(SitePathError::Empty);
        }
        if untrusted.starts_with('/') {
            return Err(SitePathError::Absolute);
        }
        if let Some(character) = untrusted.chars().find(|c| matches!(c, '\\' | ':' | '\0')) {
            return Err(SitePathError::ForbiddenCharacter { character });
        }
        let mut relative = PathBuf::new();
        for segment in untrusted.split('/') {
            match segment {
                "" => return Err(SitePathError::EmptySegment),
                "." | ".." => {
                    return Err(SitePathError::DotSegment {
                        segment: segment.to_owned(),
                    });
                }
                _ => relative.push(segment),
            }
        }
        #[expect(
            clippy::disallowed_methods,
            reason = "the one join, after every segment of the untrusted text was checked (BL-SEC-4)"
        )]
        let full = root.join(&relative);
        Ok(Self {
            root: root.to_path_buf(),
            relative,
            full,
        })
    }

    /// The whole path, root included, as built. Use [`SitePath::resolve`]
    /// before opening it.
    pub fn as_path(&self) -> &Path {
        &self.full
    }

    /// The part inside the site folder, such as `posts/why-links-matter.md`.
    pub fn relative(&self) -> &Path {
        &self.relative
    }

    /// Follows symlinks on disk and returns the real path, if it is still
    /// inside the real site folder.
    ///
    /// The file itself need not exist yet: the deepest part of the path that
    /// exists is resolved and the rest is added to it, so a new file can be
    /// written into an existing folder.
    ///
    /// # Errors
    ///
    /// Returns [`SitePathError::EscapesRoot`] when a symlink leads outside the
    /// site folder, and [`SitePathError::Io`] when the disk cannot be read.
    pub fn resolve(&self) -> Result<PathBuf, SitePathError> {
        let root = fs::canonicalize(&self.root).map_err(|source| SitePathError::Io { source })?;
        let mut existing = self.full.as_path();
        let mut missing: Vec<&OsStr> = Vec::new();
        loop {
            match fs::symlink_metadata(existing) {
                Ok(_) => break,
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    let (Some(name), Some(parent)) = (existing.file_name(), existing.parent())
                    else {
                        return Err(SitePathError::Io { source: error });
                    };
                    missing.push(name);
                    existing = parent;
                }
                Err(source) => return Err(SitePathError::Io { source }),
            }
        }
        let mut resolved =
            fs::canonicalize(existing).map_err(|source| SitePathError::Io { source })?;
        if !resolved.starts_with(&root) {
            return Err(SitePathError::EscapesRoot {
                relative: self.relative.clone(),
            });
        }
        resolved.extend(missing.iter().rev());
        Ok(resolved)
    }
}

/// Why a path was refused.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SitePathError {
    /// The text was empty.
    #[error("the path is empty")]
    Empty,
    /// The text started with `/`.
    #[error("the path starts with /, and only paths inside the site folder are allowed")]
    Absolute,
    /// The text had `//` or ended with `/`.
    #[error("the path has an empty part, from // or a trailing /")]
    EmptySegment,
    /// A segment was `.` or `..`.
    #[error("the path has a {segment:?} part, which could point outside the site folder")]
    DotSegment {
        /// The segment, `.` or `..`.
        segment: String,
    },
    /// The text had a backslash, a colon or a NUL byte.
    #[error("the path has {character:?}, which is not allowed in a site path")]
    ForbiddenCharacter {
        /// The first character that is not allowed.
        character: char,
    },
    /// A symlink on disk leads outside the site folder.
    #[error("{} leads outside the site folder through a symlink", relative.display())]
    EscapesRoot {
        /// The path inside the site folder that was asked for.
        relative: PathBuf,
    },
    /// The disk could not be read while resolving the path.
    #[error("the path could not be resolved on disk")]
    Io {
        /// What the file system reported.
        #[source]
        source: io::Error,
    },
}
