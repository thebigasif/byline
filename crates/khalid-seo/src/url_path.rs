//! [`UrlPath`]: a clean absolute path inside a site.

use std::fmt;

use crate::SeoError;

/// A clean absolute path such as `/blog/` or `/why-internal-links-matter/`.
///
/// It starts with `/`, has no empty, `.` or `..` segment, and holds no query,
/// fragment, backslash, space or control character. Letters from any script
/// are allowed; they are percent-encoded when the path becomes a URL.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UrlPath(String);

impl UrlPath {
    /// The root path, `/`.
    pub fn root() -> Self {
        Self("/".to_owned())
    }

    /// Checks `text` and wraps it.
    ///
    /// # Errors
    ///
    /// [`SeoError::Path`] when the path does not start with `/`, has an empty,
    /// `.` or `..` segment, or contains `?`, `#`, `\`, whitespace or a control
    /// character.
    pub fn try_new(text: &str) -> Result<Self, SeoError> {
        let problem = if !text.starts_with('/') {
            Some("does not start with /")
        } else if text.contains("//") {
            Some("has an empty segment, //")
        } else if text
            .split('/')
            .any(|segment| segment == "." || segment == "..")
        {
            Some("has a . or .. segment")
        } else if text
            .chars()
            .any(|c| matches!(c, '?' | '#' | '\\') || c.is_whitespace() || c.is_control())
        {
            Some("contains ?, #, \\, a space or a control character")
        } else {
            None
        };
        match problem {
            Some(problem) => Err(SeoError::Path {
                path: text.to_owned(),
                problem,
            }),
            None => Ok(Self(text.to_owned())),
        }
    }

    /// The path as written.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The path without its trailing slashes: empty for the root.
    pub(crate) fn without_trailing_slash(&self) -> &str {
        self.0.trim_end_matches('/')
    }
}

impl fmt::Display for UrlPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
