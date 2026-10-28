//! The idle memory measure waits for the server to settle, then takes five
//! readings one second apart and reports the lowest, so the allocator's
//! own schedule for handing pages back cannot swing the budget.

use khalid_perf::{FLOOR, IdleMemory, MemorySplit, STABLE, is_settled};

const STATUS: &str = "Name:\tkhalid\nVmRSS:\t   68608 kB\nRssAnon:\t   51200 kB\n\
                      RssFile:\t   17408 kB\nRssShmem:\t       0 kB\nThreads:\t9\n";

const ROLLUP: &str = "Rss:               68608 kB\nAnonymous:         51200 kB\n\
                      AnonHugePages:     12288 kB\nSwap:                  0 kB\n";

#[test]
fn the_split_is_read_from_the_status_and_the_rollup() {
    let split = MemorySplit::parse(STATUS, ROLLUP).unwrap();
    assert_eq!(split.anonymous_mb(), 50.0);
    assert_eq!(split.files_mb(), 17.0);
    assert_eq!(split.huge_pages_mb(), 12.0);
}

#[test]
fn a_status_without_the_split_gives_nothing() {
    assert_eq!(MemorySplit::parse("Name:\tkhalid\n", ROLLUP), None);
    let without_huge = MemorySplit::parse(STATUS, "Rss: 68608 kB\n").unwrap();
    assert_eq!(without_huge.huge_pages_mb(), 0.0, "no huge pages listed");
}

#[test]
fn the_measure_is_the_lowest_reading_after_the_settle() {
    let split = MemorySplit::parse(STATUS, ROLLUP);
    // Three readings before the settle: the second one is lower than the
    // measure must ever be allowed to pick, because it came before the
    // server settled.
    let idle = IdleMemory::settled_at(
        vec![80.25, 79.5, 78.0, 55.4, 55.2, 55.3, 55.1, 55.2],
        2,
        split,
    );
    assert_eq!(idle.resident_mb(), Some(55.1));
}

#[test]
fn the_report_names_where_it_settled_and_each_second() {
    let split = MemorySplit::parse(STATUS, ROLLUP);
    let idle = IdleMemory::settled_at(vec![80.25, 79.5, 55.4, 55.2, 55.3], 1, split);
    assert_eq!(idle.resident_mb(), Some(55.2));
    assert_eq!(
        idle.to_string(),
        "Idle memory 55.20 MB, the lowest of the readings after second 2: 50.00 MB \
         anonymous, 12.00 MB of it in huge pages, and 17.00 MB of code and mapped \
         files.\n\
         Resident memory each second after the first answer, in MB: 80.25 79.50 55.40 55.20 55.30\n"
    );
}

#[test]
fn a_report_without_a_split_still_names_the_settle() {
    let idle = IdleMemory::settled_at(vec![60.0], 0, None);
    assert_eq!(
        idle.to_string(),
        "Idle memory 60.00 MB, the lowest of the readings after second 1.\n\
         Resident memory each second after the first answer, in MB: 60.00\n"
    );
    assert_eq!(
        IdleMemory::settled_at(Vec::new(), 0, None).resident_mb(),
        None
    );
}

#[test]
fn a_server_settles_when_threads_hold_and_memory_stops_falling() {
    // Fifteen seconds in: the threads held at 9 for the last three and the
    // resident reading only drifts up and down within a hair. Settled.
    let mut trace = Vec::new();
    for _ in 0..FLOOR - STABLE {
        trace.push((Some(11), 72.0));
    }
    for resident in [55.4, 55.5, 55.4] {
        trace.push((Some(9), resident));
    }
    assert!(
        is_settled(&trace),
        "threads steady and memory flat: settled"
    );
}

#[test]
fn a_falling_reading_or_moving_threads_keep_it_waiting() {
    // The trace is long enough, but the resident reading is still falling
    // hard each second, as it does while tokio's blocking threads end.
    let mut trace = Vec::new();
    for _ in 0..FLOOR - STABLE {
        trace.push((Some(11), 80.0));
    }
    for resident in [72.0, 65.0, 60.0] {
        trace.push((Some(9), resident));
    }
    assert!(!is_settled(&trace), "memory still falling: not settled");

    // The resident reading is flat but the thread count changed a second
    // ago: a thread just ended, and its memory may follow.
    let mut trace = Vec::new();
    for _ in 0..FLOOR - STABLE {
        trace.push((Some(11), 80.0));
    }
    for (threads, resident) in [(Some(11), 55.4), (Some(10), 55.4), (Some(9), 55.4)] {
        trace.push((threads, resident));
    }
    assert!(!is_settled(&trace), "threads still ending: not settled");
}

#[test]
fn nothing_settles_before_the_floor() {
    // However flat the readings, the wait outlasts tokio's blocking
    // threads, which end ten seconds after their last task, so a plateau
    // before the floor never passes for settled.
    let mut trace = Vec::new();
    for _ in 0..FLOOR - 1 {
        trace.push((Some(9), 55.0));
    }
    assert!(!is_settled(&trace), "before the floor: not settled");
}

#[test]
fn a_platform_without_a_thread_count_settles_on_memory_alone() {
    let mut trace = Vec::new();
    for _ in 0..FLOOR - STABLE {
        trace.push((None, 72.0));
    }
    for resident in [55.4, 55.5, 55.4] {
        trace.push((None, resident));
    }
    assert!(
        is_settled(&trace),
        "with no thread count to watch, flat memory settles"
    );
}
