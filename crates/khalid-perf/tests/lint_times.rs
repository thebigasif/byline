//! The lint measure times posts spread over the whole site and reports the
//! median against the "Lint one post" budget.

use chrono::{TimeZone, Utc};
use khalid_core::{FixedClock, SystemClock};
use khalid_lint::{LintConfig, Linter};
use khalid_perf::{Fixture, LintTimes, Prepared};

#[cfg(test)]
fn prepared(posts: usize) -> Prepared {
    let dir = tempfile::tempdir().unwrap();
    Fixture::new(posts).unwrap().write(dir.path()).unwrap();
    Prepared::read(dir.path()).unwrap()
}

#[test]
fn each_sampled_post_is_linted_and_timed_once() {
    let site = prepared(12);
    let linter = Linter::new(&LintConfig::default(), &SystemClock).unwrap();
    let times = LintTimes::measure(&site, &linter, 5, &SystemClock);
    assert_eq!(times.samples(), 5);
    assert!(times.median_ms() > 0.0);
}

#[test]
fn asking_for_more_samples_than_posts_lints_each_post_once() {
    let site = prepared(3);
    let linter = Linter::new(&LintConfig::default(), &SystemClock).unwrap();
    let times = LintTimes::measure(&site, &linter, 100, &SystemClock);
    assert_eq!(times.samples(), 3);
}

#[test]
fn a_stopped_clock_measures_nothing_as_zero() {
    let site = prepared(4);
    let stopped = FixedClock::new(Utc.with_ymd_and_hms(2026, 10, 31, 9, 0, 0).unwrap());
    let linter = Linter::new(&LintConfig::default(), &stopped).unwrap();
    let times = LintTimes::measure(&site, &linter, 4, &stopped);
    assert_eq!(times.median_ms(), 0.0);
}

#[test]
fn the_json_names_the_baseline_measure() {
    let json = LintTimes::from_ms(vec![30.0, 10.0, 20.0]).to_json();
    assert_eq!(json, "{\"lint_one_post_ms\": 20.0000}\n");
}
