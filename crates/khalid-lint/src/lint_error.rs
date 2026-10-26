//! The one error type of this crate (BL-ERR-1).

/// Something that stops the linter from starting.
///
/// The binary turns each variant into a message with what, why and fix
/// (BL-ERR-4).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum LintError {
    /// The catalogue built into the crate is malformed.
    #[error("the built-in message catalogue is malformed")]
    Catalogue {
        /// What the TOML reader found.
        #[source]
        source: toml::de::Error,
    },
}
