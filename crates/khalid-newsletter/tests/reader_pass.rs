//! The reader pass: the signed cookie that lets a confirmed subscriber read
//! gated content, and the signed link that gives a returning subscriber a
//! new one (G41, task 5.11).

use std::sync::Arc;

use chrono::{DateTime, TimeDelta, Utc};
use khalid_core::{AbsoluteUrl, Clock, FixedClock, Secret};
use khalid_newsletter::{PASS_DAYS, ReaderPass, SiteLinks};

#[cfg(test)]
fn at(seconds: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(1_790_000_000 + seconds, 0).unwrap()
}

#[cfg(test)]
fn pass(key: u8, address: &str, clock: impl Clock + 'static) -> ReaderPass {
    let site = SiteLinks::new(&AbsoluteUrl::parse(address).unwrap(), "/blog/");
    ReaderPass::new(&Secret::new([key; 32]), &site, Arc::new(clock))
}

/// The `name=value` pair of a `Set-Cookie` value, as a browser sends it back.
#[cfg(test)]
fn sent(set_cookie: &str) -> String {
    set_cookie.split(';').next().unwrap().to_owned()
}

#[test]
fn the_cookie_is_sent_to_the_whole_blog_and_hidden_from_scripts() {
    let cookie = pass(7, "https://example.com", FixedClock::new(at(0))).cookie();
    assert!(cookie.starts_with("khalid_reader="), "{cookie}");
    assert!(cookie.contains("; Path=/blog/"), "{cookie}");
    assert!(cookie.contains("; HttpOnly; SameSite=Lax; Secure"), "{cookie}");
    let max_age = format!("; Max-Age={}", TimeDelta::days(PASS_DAYS).num_seconds());
    assert!(cookie.contains(&max_age), "{cookie}");
    let plain = pass(7, "http://localhost:8080", FixedClock::new(at(0))).cookie();
    assert!(!plain.contains("Secure"), "{plain}");
}

#[test]
fn a_cookie_the_site_made_admits_until_it_runs_out() {
    let cookie = sent(&pass(7, "https://example.com", FixedClock::new(at(0))).cookie());
    let header = format!("theme=light; {cookie}");
    let later = TimeDelta::days(PASS_DAYS).num_seconds();
    assert!(pass(7, "https://example.com", FixedClock::new(at(later - 1))).admits(&header));
    assert!(!pass(7, "https://example.com", FixedClock::new(at(later))).admits(&header));
}

#[test]
fn a_cookie_made_with_another_key_or_changed_admits_no_one() {
    let checker = pass(7, "https://example.com", FixedClock::new(at(0)));
    let other = sent(&pass(8, "https://example.com", FixedClock::new(at(0))).cookie());
    assert!(!checker.admits(&other));
    let mine = sent(&checker.cookie());
    let (until, sig) = mine.split_once('.').unwrap();
    let longer = format!("{until}9.{sig}");
    assert!(!checker.admits(&longer));
    assert!(!checker.admits("khalid_reader=nonsense"));
    assert!(!checker.admits(""));
}

#[test]
fn a_pass_link_opens_for_two_days_and_is_no_cookie() {
    let maker = pass(7, "https://example.com", FixedClock::new(at(0)));
    let link = maker.link("/blog/guides/checklist/");
    let code = link
        .strip_prefix("https://example.com/blog/subscribe/pass?code=")
        .unwrap();
    let (code, back) = code.split_once("&back=").unwrap();
    assert_eq!(back, "%2Fblog%2Fguides%2Fchecklist%2F");
    let two_days = TimeDelta::days(2).num_seconds();
    assert!(pass(7, "https://example.com", FixedClock::new(at(two_days - 1))).opens(code));
    assert!(!pass(7, "https://example.com", FixedClock::new(at(two_days))).opens(code));
    assert!(!maker.admits(&format!("khalid_reader={code}")));
    let cookie = sent(&maker.cookie());
    assert!(!maker.opens(cookie.trim_start_matches("khalid_reader=")));
}
