//! Each network address may send a few comments in a short time and a
//! few more in a day.

use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{DateTime, TimeDelta, TimeZone, Utc};
use khalid_comments::{RateLimitError, RateLimits};
use khalid_core::Clock;

/// A clock the test moves forward by hand.
#[derive(Debug, Clone)]
struct Moving(Arc<Mutex<DateTime<Utc>>>);

#[cfg(test)]
impl Moving {
    fn at(hour: u32, minute: u32) -> Self {
        let start = Utc.with_ymd_and_hms(2026, 10, 27, hour, minute, 0).unwrap();
        Self(Arc::new(Mutex::new(start)))
    }

    fn forward(&self, minutes: i64) {
        *self.0.lock().unwrap() += TimeDelta::minutes(minutes);
    }
}

#[cfg(test)]
impl Clock for Moving {
    fn now(&self) -> DateTime<Utc> {
        *self.0.lock().unwrap()
    }
}

#[cfg(test)]
fn address(text: &str) -> IpAddr {
    text.parse().unwrap()
}

#[cfg(test)]
fn waits(result: Result<(), RateLimitError>) -> Duration {
    match result {
        Err(RateLimitError::Limited { retry_after }) => retry_after,
        other => panic!("expected a limit, got {other:?}"),
    }
}

#[test]
fn a_burst_is_stopped_and_the_wait_is_told() {
    let clock = Moving::at(9, 0);
    let limits = RateLimits::new(clock.clone());
    let reader = address("203.0.113.7");
    for _ in 0..RateLimits::BURST {
        limits.check(reader).unwrap();
        clock.forward(1);
    }
    // Three comments at 9:00, 9:01 and 9:02; the first leaves the
    // ten-minute window at 9:10.
    assert_eq!(waits(limits.check(reader)), Duration::from_mins(7));
    // Another reader is not held up.
    limits.check(address("198.51.100.4")).unwrap();
    clock.forward(7);
    limits.check(reader).unwrap();
}

#[test]
fn a_refused_comment_does_not_count() {
    let clock = Moving::at(9, 0);
    let limits = RateLimits::new(clock.clone());
    let reader = address("203.0.113.7");
    for _ in 0..RateLimits::BURST {
        limits.check(reader).unwrap();
    }
    for _ in 0..5 {
        assert!(limits.check(reader).is_err());
    }
    clock.forward(10);
    limits.check(reader).unwrap();
}

#[test]
fn a_day_has_its_own_limit_until_midnight() {
    let clock = Moving::at(12, 0);
    let limits = RateLimits::new(clock.clone());
    let reader = address("203.0.113.7");
    for _ in 0..RateLimits::DAILY {
        limits.check(reader).unwrap();
        clock.forward(11);
    }
    // 12:00 plus 10 times 11 minutes is 13:50; midnight is 10:10 away.
    assert_eq!(waits(limits.check(reader)), Duration::from_mins(610));
    // A new day brings a new salt, and the old hashes are dropped.
    clock.forward(610);
    limits.check(reader).unwrap();
}

#[test]
fn addresses_in_one_ipv6_network_share_a_limit() {
    let limits = RateLimits::new(Moving::at(9, 0));
    for n in 0..RateLimits::BURST {
        limits
            .check(address(&format!("2001:db8:1:2::{n}")))
            .unwrap();
    }
    assert!(limits.check(address("2001:db8:1:2:ffff::9")).is_err());
    limits.check(address("2001:db8:1:3::1")).unwrap();
}

#[test]
fn an_ipv4_address_inside_ipv6_is_the_same_reader() {
    let limits = RateLimits::new(Moving::at(9, 0));
    for _ in 0..RateLimits::BURST {
        limits.check(address("203.0.113.7")).unwrap();
    }
    assert!(limits.check(address("::ffff:203.0.113.7")).is_err());
}
