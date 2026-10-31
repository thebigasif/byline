//! The audits a site has run, kept for the score's trend.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{Report, Severity};

/// One audit as the trend keeps it: when it ran, its score and its counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Run {
    /// When the audit ran, in UTC.
    pub at: DateTime<Utc>,
    /// The health score out of 100.
    pub score: u8,
    /// How many pages it read.
    pub pages: usize,
    /// How many errors it found.
    pub errors: usize,
    /// How many warnings it found.
    pub warnings: usize,
    /// How many notices it found.
    pub notices: usize,
}

impl Run {
    /// The run of `report`, made `at` that moment.
    pub fn of(report: &Report, at: DateTime<Utc>) -> Self {
        Self {
            at,
            score: report.score(),
            pages: report.pages(),
            errors: report.count(Severity::Error),
            warnings: report.count(Severity::Warning),
            notices: report.count(Severity::Notice),
        }
    }

    /// The run as one line of JSON, ending in a line break, to add to the
    /// history file.
    ///
    /// # Errors
    ///
    /// Returns the `serde_json` error, which a run of plain numbers and a
    /// time never causes.
    pub fn line(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self).map(|line| format!("{line}\n"))
    }
}

/// Every run in a history file's `text`, oldest first, one JSON object per
/// line. A line that does not read as a run, such as one cut short when a
/// disk filled, is left out, so one bad line never hides the trend.
pub fn history(text: &str) -> Vec<Run> {
    text.lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}
