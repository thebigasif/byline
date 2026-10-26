//! The real clock moves forward, and the fixed clock never does.

use byline_core::{Clock, FixedClock, SystemClock};
use chrono::{TimeZone, Utc};

#[test]
fn the_system_clock_reads_the_present() {
    let start_of_this_project = Utc.with_ymd_and_hms(2026, 10, 26, 0, 0, 0).unwrap();
    let first = SystemClock.now();
    let second = SystemClock.now();
    assert!(first >= start_of_this_project, "the clock reads {first}");
    assert!(
        second >= first,
        "the clock went back from {first} to {second}"
    );
}

#[test]
fn a_fixed_clock_always_reads_the_same_time() {
    let launch = Utc.with_ymd_and_hms(2026, 10, 26, 9, 30, 0).unwrap();
    let clock = FixedClock::new(launch);
    assert_eq!(clock.now(), launch);
    assert_eq!(clock.now(), launch);
}

#[test]
fn code_can_take_any_clock_behind_a_reference() {
    let launch = Utc.with_ymd_and_hms(2026, 10, 26, 9, 30, 0).unwrap();
    let clocks: [&dyn Clock; 2] = [&SystemClock, &FixedClock::new(launch)];
    assert!(clocks.iter().all(|clock| clock.now() >= launch));
}
