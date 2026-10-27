//! Slows down guessing: too many failed sign-ins for one email or from one
//! address are refused for a while (RULES.md BL-SEC-5).

use std::collections::BTreeMap;
use std::net::IpAddr;
use std::sync::Mutex;

use chrono::{DateTime, TimeDelta, Utc};

use crate::Email;

/// Failed sign-ins allowed for one email or one address in [`WINDOW`].
pub const MAX_FAILURES: usize = 5;

/// How far back failures count.
pub const WINDOW: TimeDelta = TimeDelta::minutes(15);

/// The most emails and addresses remembered at once, so a flood of made-up
/// emails cannot fill the memory. When full, the oldest is forgotten.
const MAX_ENTRIES: usize = 10_000;

/// What failures are counted against.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Against {
    Email(Email),
    Address(IpAddr),
}

/// Recent failed sign-ins, kept in memory. A restart forgets them, which
/// costs a guesser nothing they could not get by waiting.
#[derive(Debug, Default)]
pub struct SignInLimiter {
    failures: Mutex<BTreeMap<Against, Vec<DateTime<Utc>>>>,
}

impl SignInLimiter {
    /// No failures yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// When the next attempt for `email` from `from` may be made, if it is
    /// refused at `now`.
    pub fn blocked_until(
        &self,
        email: &Email,
        from: Option<IpAddr>,
        now: DateTime<Utc>,
    ) -> Option<DateTime<Utc>> {
        let failures = self.lock()?;
        keys(email, from)
            .filter_map(|key| {
                let recent: Vec<&DateTime<Utc>> = failures
                    .get(&key)?
                    .iter()
                    .filter(|at| **at + WINDOW > now)
                    .collect();
                let oldest = recent.first()?;
                (recent.len() >= MAX_FAILURES).then(|| **oldest + WINDOW)
            })
            .max()
    }

    /// Counts a failed attempt at `now`.
    pub fn failed(&self, email: &Email, from: Option<IpAddr>, now: DateTime<Utc>) {
        let Some(mut failures) = self.lock() else {
            return;
        };
        failures.retain(|_, times| times.last().is_some_and(|at| *at + WINDOW > now));
        for key in keys(email, from) {
            if failures.len() >= MAX_ENTRIES && !failures.contains_key(&key) {
                let oldest = failures
                    .iter()
                    .min_by_key(|(_, times)| times.last().copied())
                    .map(|(key, _)| key.clone());
                if let Some(oldest) = oldest {
                    failures.remove(&oldest);
                }
            }
            let times = failures.entry(key).or_default();
            times.retain(|at| *at + WINDOW > now);
            times.push(now);
        }
    }

    /// Forgets the failures for `email` after it signs in.
    pub fn succeeded(&self, email: &Email) {
        if let Some(mut failures) = self.lock() {
            failures.remove(&Against::Email(email.clone()));
        }
    }

    /// The failures, or `None` if a thread panicked while holding them. The
    /// limiter then lets attempts through rather than lock everyone out;
    /// the password check still stands.
    fn lock(&self) -> Option<std::sync::MutexGuard<'_, BTreeMap<Against, Vec<DateTime<Utc>>>>> {
        self.failures.lock().ok()
    }
}

fn keys(email: &Email, from: Option<IpAddr>) -> impl Iterator<Item = Against> {
    std::iter::once(Against::Email(email.clone())).chain(from.map(Against::Address))
}
