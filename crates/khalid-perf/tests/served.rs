//! The serve measures read the server's address and memory correctly, and
//! address a generated post where the site serves it.

use khalid_perf::{Fixture, ServeTimes, announced, resident_mb};

#[test]
fn the_json_names_the_baseline_measures() {
    let json = ServeTimes::new(42.5, 37.25, 18.0).to_json();
    assert_eq!(
        json,
        "{\"cold_start_first_byte_ms\": 42.50, \"idle_memory_mb\": 37.25, \
         \"incremental_rebuild_ms\": 18.00}\n"
    );
}

#[test]
fn the_announced_line_gives_the_host_and_the_mount() {
    let line = "{\"address\":\"http://127.0.0.1:41873/blog/\"}\n";
    assert_eq!(
        announced(line),
        Some(("127.0.0.1:41873".to_owned(), "/blog/".to_owned()))
    );
}

#[test]
fn a_line_without_an_address_gives_nothing() {
    assert_eq!(announced("{\"error\":\"the port is taken\"}\n"), None);
    assert_eq!(announced(""), None);
}

#[test]
fn resident_memory_is_read_in_megabytes() {
    let status = "Name:\tkhalid\nVmPeak:\t  90000 kB\nVmRSS:\t   51200 kB\nThreads:\t9\n";
    assert_eq!(resident_mb(status), Some(50.0));
    assert_eq!(resident_mb("Name:\tkhalid\n"), None);
}

#[test]
fn a_post_is_addressed_by_its_slug_under_the_mount() {
    let fixture = Fixture::THOUSAND;
    let text = fixture.post_text(0);
    let slug = text
        .lines()
        .find_map(|line| line.strip_prefix("slug: "))
        .unwrap();
    assert_eq!(fixture.post_address(0), format!("/blog/{slug}/"));
}
