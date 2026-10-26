//! The one error type of this crate (BL-ERR-1).

use crate::RuleId;

/// Something that stops the linter from starting.
///
/// The binary turns each variant into a message with what, why and fix
/// (BL-ERR-4). Lines and columns count from 1.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum LintError {
    /// `lint.toml` is not valid TOML, or holds a key or value the linter
    /// does not know.
    #[error("lint.toml line {line}, column {column}: {}", source.message())]
    Syntax {
        /// The line of the mistake.
        line: usize,
        /// The column of the mistake.
        column: usize,
        /// What the TOML reader found.
        #[source]
        source: toml::de::Error,
    },
    /// `lint.toml` sets a severity for a rule that does not exist.
    #[error("lint.toml line {line}, column {column}: there is no rule called {rule:?}")]
    UnknownRule {
        /// The rule name, as written.
        rule: String,
        /// The line of the rule name.
        line: usize,
        /// The column of the rule name.
        column: usize,
    },
    /// A rule has no message in the catalogue.
    #[error("the rule {rule} has no message in the catalogue")]
    MissingMessage {
        /// The rule without a message.
        rule: RuleId,
    },
    /// The catalogue built into the crate is malformed.
    #[error("the built-in message catalogue is malformed")]
    Catalogue {
        /// What the TOML reader found.
        #[source]
        source: toml::de::Error,
    },
}
