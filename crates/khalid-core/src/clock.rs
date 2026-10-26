//! The one way Khalid reads the time.

use std::fmt;

use chrono::{DateTime, Utc};

/// A source of the current time, in UTC.
///
/// Code that needs the time takes a `&dyn Clock` or a `C: Clock` instead of
/// reading the system clock itself (RULES.md BL-RUN-1). Production passes
/// [`SystemClock`]. Tests pass a `FixedClock`, from the `test-util` feature, so
/// a result never depends on when the test runs (BL-TST-2).
pub trait Clock: fmt::Debug + Send + Sync {
    /// The current time.
    fn now(&self) -> DateTime<Utc>;
}

/// The computer's own clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        #[expect(
            clippy::disallowed_methods,
            reason = "the one place that reads the system clock (BL-RUN-1)"
        )]
        let now = Utc::now();
        now
    }
}

/// A clock that always reads the same time, for tests.
///
/// # Examples
///
/// ```
/// use khalid_core::{Clock, FixedClock};
/// use chrono::{TimeZone, Utc};
///
/// let launch = Utc.with_ymd_and_hms(2026, 10, 26, 9, 0, 0).single().ok_or("no such time")?;
/// let clock = FixedClock::new(launch);
/// assert_eq!(clock.now(), launch);
/// # Ok::<(), &str>(())
/// ```
#[cfg(any(test, feature = "test-util"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedClock(DateTime<Utc>);

#[cfg(any(test, feature = "test-util"))]
impl FixedClock {
    /// A clock stopped at `at`.
    pub fn new(at: DateTime<Utc>) -> Self {
        Self(at)
    }
}

#[cfg(any(test, feature = "test-util"))]
impl Clock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        self.0
    }
}
