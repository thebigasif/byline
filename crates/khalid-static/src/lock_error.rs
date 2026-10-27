//! What can go wrong while reading or locking a page.

/// Why a page could not be read or locked.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum LockError {
    /// A built-in notes file is not valid. The crate's tests rule this out.
    #[error("the static mode notes for {language} cannot be read")]
    Notes {
        /// The file's language, such as `de`.
        language: String,
        /// What the TOML reader found.
        #[source]
        source: toml::de::Error,
    },
    /// A form starts but never ends, so the page cannot be locked safely.
    #[error("the form at byte {offset} has no closing </form> tag")]
    Unclosed {
        /// Where the form's opening tag starts in the page.
        offset: usize,
    },
}
