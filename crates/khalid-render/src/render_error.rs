//! What can go wrong while loading or rendering templates.

/// Why a template could not be loaded or a page could not be rendered.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RenderError {
    /// A built-in strings file is not valid TOML. A build mistake, caught by
    /// this crate's tests.
    #[error("the built-in strings file for {language} is broken")]
    Strings {
        /// The language of the broken file.
        language: String,
        /// What the TOML parser reported.
        #[source]
        source: toml::de::Error,
    },
}
