//! Why `theme/tokens.toml` could not be read.

/// Why `theme/tokens.toml` could not be read.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TokensError {
    /// A value is not a colour, a font list or a length, or a field is
    /// one the renderer does not know.
    #[error("theme/tokens.toml has a mistake")]
    Parse {
        /// The line of the mistake, counting from 1, when it is known.
        line: Option<usize>,
        /// What the TOML reader reported.
        #[source]
        source: toml::de::Error,
    },
}

impl TokensError {
    /// The line of a mistake in the file, when there is one.
    pub fn line(&self) -> Option<usize> {
        match self {
            Self::Parse { line, .. } => *line,
        }
    }
}
