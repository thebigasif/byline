//! Settings, read once when Byline starts.

/// Every setting Byline reads, checked once at start and then passed by
/// reference (RULES.md BL-RUN-2).
///
/// It has no fields yet. Phase 1 fills it from `site.toml`, the environment and
/// the command line, and no other module reads those sources itself.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Config {}

impl Config {
    /// The settings Byline uses when nothing is configured.
    pub fn new() -> Self {
        Self::default()
    }
}
