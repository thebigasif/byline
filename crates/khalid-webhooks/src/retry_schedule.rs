//! When a failed delivery is tried again.

use std::time::Duration;

use chrono::{DateTime, TimeDelta, Utc};

/// The waits after each failed attempt. Seven attempts in all, the last 23
/// hours and 36 minutes after the first, so a receiver that is down for most
/// of a day still gets the event (`docs/standards/api.md`).
const DEFAULT_DELAYS: [Duration; 6] = [
    Duration::from_mins(1),
    Duration::from_mins(5),
    Duration::from_mins(30),
    Duration::from_hours(2),
    Duration::from_hours(5),
    Duration::from_hours(16),
];

/// The backoff between delivery attempts: growing waits, so a receiver that
/// is briefly down gets the event within minutes and one down for hours is
/// not flooded. No random jitter, so the log is predictable (BL-TST-2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetrySchedule {
    delays: Vec<Duration>,
}

impl RetrySchedule {
    /// The waits after each failure, in order.
    pub fn delays(&self) -> &[Duration] {
        &self.delays
    }

    /// When to try again after `failures` failed attempts, the last at
    /// `last`. `None` means give up: no retries left, or none failed.
    pub fn next(&self, failures: usize, last: DateTime<Utc>) -> Option<DateTime<Utc>> {
        let wait = self.delays.get(failures.checked_sub(1)?)?;
        last.checked_add_signed(TimeDelta::from_std(*wait).ok()?)
    }
}

impl Default for RetrySchedule {
    fn default() -> Self {
        Self {
            delays: DEFAULT_DELAYS.to_vec(),
        }
    }
}
