//! Five failed sign-ins for one email, or from one address, in fifteen
//! minutes stop the next attempt until the oldest failure is old enough.

use std::net::{IpAddr, Ipv4Addr};

use chrono::{DateTime, TimeDelta, TimeZone, Utc};
use khalid_auth::{Email, MAX_FAILURES, SignInLimiter, WINDOW};

#[cfg(test)]
fn start() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0)
        .single()
        .expect("a time")
}

#[cfg(test)]
fn email(text: &str) -> Email {
    text.parse().expect("an email address")
}

const HOME: Option<IpAddr> = Some(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7)));
const ELSEWHERE: Option<IpAddr> = Some(IpAddr::V4(Ipv4Addr::new(198, 51, 100, 9)));

#[test]
fn the_fifth_failure_for_an_email_blocks_it_from_any_address_until_the_window_passes() {
    let limiter = SignInLimiter::new();
    let ana = email("ana@example.com");
    for minute in 0..4 {
        let at = start() + TimeDelta::minutes(minute);
        limiter.failed(&ana, HOME, at);
        assert_eq!(
            limiter.blocked_until(&ana, HOME, at),
            None,
            "after {} failures",
            minute + 1
        );
    }
    limiter.failed(&ana, HOME, start() + TimeDelta::minutes(4));
    let later = start() + TimeDelta::minutes(5);
    assert_eq!(
        limiter.blocked_until(&ana, ELSEWHERE, later),
        Some(start() + WINDOW)
    );
    assert_eq!(
        limiter.blocked_until(&ana, ELSEWHERE, start() + WINDOW),
        None
    );
}

#[test]
fn one_address_trying_many_emails_is_blocked_for_all_of_them() {
    let limiter = SignInLimiter::new();
    for n in 0..MAX_FAILURES {
        limiter.failed(&email(&format!("guess{n}@example.com")), HOME, start());
    }
    let fresh = email("someone@example.com");
    assert!(limiter.blocked_until(&fresh, HOME, start()).is_some());
    assert_eq!(limiter.blocked_until(&fresh, ELSEWHERE, start()), None);
}

#[test]
fn signing_in_forgets_the_failures_for_that_email() {
    let limiter = SignInLimiter::new();
    let ana = email("ana@example.com");
    for _ in 0..MAX_FAILURES - 1 {
        limiter.failed(&ana, None, start());
    }
    limiter.succeeded(&ana);
    limiter.failed(&ana, None, start());
    assert_eq!(limiter.blocked_until(&ana, None, start()), None);
}
