//! Settings, read once when Khalid starts.

/// Every setting Khalid reads, checked once at start and then passed by
/// reference (RULES.md BL-RUN-2).
///
/// It has no fields yet. Phase 1 fills it from `site.toml`, the environment and
/// the command line, and no other module reads those sources itself.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Config {}

impl Config {
    /// The settings Khalid uses when nothing is configured.
    pub fn new() -> Self {
        Self::default()
    }
}
