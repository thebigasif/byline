//! The idle memory measure says where the memory sits and how it moved
//! during the wait, so a failing budget can be read from the CI log.

use khalid_perf::{IdleMemory, MemorySplit};

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
fn the_report_gives_each_second_and_the_split() {
    let split = MemorySplit::parse(STATUS, ROLLUP);
    let idle = IdleMemory::new(vec![80.25, 79.5, 67.0], split);
    assert_eq!(idle.resident_mb(), Some(67.0));
    assert_eq!(
        idle.to_string(),
        "Idle memory 67.00 MB: 50.00 MB anonymous, 12.00 MB of it in huge pages, \
         and 17.00 MB of code and mapped files.\n\
         Resident memory each second after the first answer, in MB: 80.25 79.50 67.00\n"
    );
}

#[test]
fn a_report_without_a_split_still_gives_each_second() {
    let idle = IdleMemory::new(vec![60.0], None);
    assert_eq!(
        idle.to_string(),
        "Idle memory 60.00 MB.\n\
         Resident memory each second after the first answer, in MB: 60.00\n"
    );
    assert_eq!(IdleMemory::new(Vec::new(), None).resident_mb(), None);
}
