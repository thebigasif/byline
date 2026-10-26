//! How much a rule's finding matters.

use serde::{Deserialize, Serialize};

/// What happens when a rule finds a problem, as set in `lint.toml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// The post cannot be published until the writer fixes it.
    Block,
    /// The writer sees the message, and the post can still be published.
    Warn,
    /// The rule does not run.
    Off,
}
