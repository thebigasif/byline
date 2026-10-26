//! The render measure times every page and reports its percentiles.

use chrono::{TimeZone, Utc};
use khalid_core::{FixedClock, SystemClock};
use khalid_perf::{Fixture, Prepared, RenderTimes, percentile};
use khalid_render::Renderer;

#[cfg(test)]
fn prepared(posts: usize) -> Prepared {
    let dir = tempfile::tempdir().unwrap();
    Fixture::new(posts).unwrap().write(dir.path()).unwrap();
    Prepared::read(dir.path()).unwrap()
}

#[test]
fn every_page_is_timed_once_per_measured_round() {
    let site = prepared(12);
    let renderer = Renderer::new().unwrap();
    let times = RenderTimes::measure(&site, &renderer, 3, &SystemClock).unwrap();
    assert_eq!(times.samples(), 12 * 2, "the first round only warms up");
    assert!(times.p50_ms() > 0.0);
    assert!(times.p50_ms() <= times.p99_ms());
}

#[test]
fn a_stopped_clock_measures_nothing_as_zero() {
    let site = prepared(3);
    let renderer = Renderer::new().unwrap();
    let stopped = FixedClock::new(Utc.with_ymd_and_hms(2026, 10, 26, 9, 0, 0).unwrap());
    let times = RenderTimes::measure(&site, &renderer, 2, &stopped).unwrap();
    assert_eq!((times.p50_ms(), times.p99_ms()), (0.0, 0.0));
}

#[test]
fn percentiles_pick_the_nearest_rank() {
    let sorted: Vec<f64> = (1..=100).map(f64::from).collect();
    assert_eq!(percentile(&sorted, 50), 50.0);
    assert_eq!(percentile(&sorted, 99), 99.0);
    assert_eq!(percentile(&sorted, 100), 100.0);
    assert_eq!(percentile(&[7.0], 99), 7.0);
    assert_eq!(percentile(&[], 50), 0.0);
}

#[test]
fn the_json_names_the_baseline_measures() {
    let site = prepared(2);
    let renderer = Renderer::new().unwrap();
    let times = RenderTimes::measure(&site, &renderer, 2, &SystemClock).unwrap();
    let json = times.to_json();
    assert!(json.starts_with("{\"post_render_p50_ms\": "), "{json}");
    assert!(json.contains(", \"post_render_p99_ms\": "), "{json}");
}
