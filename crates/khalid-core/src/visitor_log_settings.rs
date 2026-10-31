//! Whether `khalid serve` keeps a log of each visit, from the
//! `[visitor_log]` table of `site.toml` (GEO C1).

use serde::Deserialize;

use crate::Mode;

/// The fewest days the visitor log keeps its rows.
const FEWEST_DAYS: u8 = 7;

/// The most days the visitor log keeps its rows.
const MOST_DAYS: u8 = 14;

/// Whether the visitor log is on and how long it keeps each visit. It is
/// off unless the table switches it on, because it holds personal data
/// the site owner should have checked with a lawyer first
/// (`docs/privacy/visitor-log.md`):
///
/// ```toml
/// [visitor_log]
/// on = true
/// keep_days = 14
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct VisitorLogSettings {
    /// Whether each visit is logged, off unless set.
    #[serde(default)]
    pub on: bool,
    /// How many days a visit is kept, 14 unless set, from 7 to 14.
    #[serde(default = "most_days")]
    pub keep_days: u8,
}

impl Default for VisitorLogSettings {
    fn default() -> Self {
        Self {
            on: false,
            keep_days: MOST_DAYS,
        }
    }
}

fn most_days() -> u8 {
    MOST_DAYS
}

impl VisitorLogSettings {
    /// The key that is wrong and why, on a site run in `mode`.
    pub(crate) fn mistake(&self, mode: Mode) -> Option<(&'static str, &'static str)> {
        if !(FEWEST_DAYS..=MOST_DAYS).contains(&self.keep_days) {
            return Some((
                "keep_days",
                "the visitor log keeps its rows 7 to 14 days, so use a number from 7 to 14",
            ));
        }
        if self.on && mode == Mode::Static {
            return Some((
                "on",
                "a static host sends the pages without Khalid, so there are no visits to log; set on = false, or mode = \"server\"",
            ));
        }
        None
    }
}
