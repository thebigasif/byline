//! How often one network address may comment, kept without the address.

use std::collections::BTreeMap;
use std::net::IpAddr;
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use chrono::{DateTime, NaiveDate, TimeDelta, Utc};
use khalid_core::Clock;
use sha2::{Digest, Sha256};

/// How long the short limit looks back.
const BURST_WINDOW: TimeDelta = TimeDelta::minutes(10);

/// How many addresses are remembered at most. Past it, addresses with no
/// comment in the short window are forgotten first, so an attack from
/// many addresses cannot grow memory without end.
const MAX_ADDRESSES: usize = 50_000;

/// The comment limits per network address (RULES.md BL-SEC-5): at most
/// [`Self::BURST`] comments in ten minutes and [`Self::DAILY`] in a day.
///
/// The raw address is never kept. Each one is hashed with SHA-256 and a
/// random salt made each day (UTC), and only the hash and the times of its
/// comments are held, in memory. When the day turns, the salt and every
/// hash are dropped. An IPv6 address counts by its /64 network, which one
/// home or server usually has whole.
#[derive(Debug)]
pub struct RateLimits {
    clock: Box<dyn Clock>,
    state: Mutex<Option<Day>>,
}

/// One day's salt and the hashed addresses seen that day.
#[derive(Debug)]
struct Day {
    date: NaiveDate,
    salt: [u8; 32],
    sent: BTreeMap<[u8; 32], Vec<DateTime<Utc>>>,
}

/// Why a comment may not be sent now.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RateLimitError {
    /// The address sent too many comments; it may try again after `retry_after`.
    #[error("too many comments from one address; try again in {retry_after:?}")]
    Limited {
        /// How long until a comment is allowed again.
        retry_after: Duration,
    },
    /// The system gave no random bytes for the day's salt.
    #[error("no random bytes for the day's salt")]
    NoRandomness {
        /// What the system said.
        #[source]
        source: getrandom::Error,
    },
}

impl RateLimits {
    /// How many comments one address may send in ten minutes.
    pub const BURST: usize = 3;

    /// How many comments one address may send in one day (UTC).
    pub const DAILY: usize = 10;

    /// Limits that read the time from `clock`.
    pub fn new(clock: impl Clock + 'static) -> Self {
        Self {
            clock: Box::new(clock),
            state: Mutex::new(None),
        }
    }

    /// Counts a comment from `address` when the limits allow it.
    ///
    /// # Errors
    ///
    /// [`RateLimitError::Limited`] when `address` sent too many comments,
    /// with how long until it may send one; the refused comment is not
    /// counted. [`RateLimitError::NoRandomness`] when a new day's salt
    /// cannot be made.
    pub fn check(&self, address: IpAddr) -> Result<(), RateLimitError> {
        let now = self.clock.now();
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let today = now.date_naive();
        let day = match state.take() {
            Some(day) if day.date == today => day,
            _ => Day::new(today)?,
        };
        let day = state.insert(day);
        let key = day.hash(address);
        let sent = day.sent.get(&key).map_or(&[][..], Vec::as_slice);
        if let Some(retry_after) = wait(sent, now) {
            return Err(RateLimitError::Limited { retry_after });
        }
        if day.sent.len() >= MAX_ADDRESSES && !day.sent.contains_key(&key) {
            day.sent
                .retain(|_, times| times.iter().any(|&at| now - at < BURST_WINDOW));
        }
        day.sent.entry(key).or_default().push(now);
        Ok(())
    }
}

impl Day {
    fn new(date: NaiveDate) -> Result<Self, RateLimitError> {
        let mut salt = [0; 32];
        getrandom::fill(&mut salt).map_err(|source| RateLimitError::NoRandomness { source })?;
        Ok(Self {
            date,
            salt,
            sent: BTreeMap::new(),
        })
    }

    /// The salted hash of the network `address` is in.
    fn hash(&self, address: IpAddr) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(self.salt);
        match address.to_canonical() {
            IpAddr::V4(v4) => hasher.update(v4.octets()),
            IpAddr::V6(v6) => hasher.update(v6.octets().get(..8).unwrap_or_default()),
        }
        hasher.finalize().into()
    }
}

/// How long an address that sent comments at `sent`, oldest first, all
/// today, must wait at `now`. `None` when it may send one now.
fn wait(sent: &[DateTime<Utc>], now: DateTime<Utc>) -> Option<Duration> {
    let mut until = None;
    if sent.len() >= RateLimits::DAILY {
        until = now
            .date_naive()
            .succ_opt()?
            .and_hms_opt(0, 0, 0)?
            .and_utc()
            .into();
    }
    let recent: Vec<_> = sent.iter().filter(|&&at| now - at < BURST_WINDOW).collect();
    if recent.len() >= RateLimits::BURST {
        let first = recent.first().map(|&&at| at + BURST_WINDOW);
        until = until.max(first);
    }
    until.and_then(|at| (at - now).to_std().ok())
}
