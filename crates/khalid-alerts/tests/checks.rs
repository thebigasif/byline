//! The alert checks: 404 spikes, certificates and stale posts (KH-GRO-6).

use std::collections::BTreeMap;

use chrono::{DateTime, TimeZone, Utc};
use khalid_alerts::{
    Finding, LivePost, ServedCertificate, certificate, not_found_spike, stale_posts,
};

#[cfg(test)]
fn day(year: i32, month: u32, day: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(year, month, day, 9, 30, 0).unwrap()
}

#[cfg(test)]
fn counts(pairs: &[(&str, u64)]) -> BTreeMap<String, u64> {
    pairs
        .iter()
        .map(|(path, hits)| ((*path).to_owned(), *hits))
        .collect()
}

#[test]
fn a_spike_names_the_addresses_that_grew_most_in_that_hour() {
    let before = counts(&[("/blog/old/", 10), ("/blog/quiet/", 3)]);
    let now = counts(&[
        ("/blog/old/", 40),
        ("/blog/quiet/", 3),
        ("/wp-login.php", 25),
    ]);
    let found = not_found_spike(&before, &now, 50, day(2026, 11, 3));
    assert_eq!(
        found,
        [
            Finding {
                key: "not-found-spike:2026-11-03T09:/blog/old/".to_owned(),
                line: "/blog/old/: 30 more 404s".to_owned(),
            },
            Finding {
                key: "not-found-spike:2026-11-03T09:/wp-login.php".to_owned(),
                line: "/wp-login.php: 25 more 404s".to_owned(),
            },
        ]
    );
    let under = not_found_spike(&before, &now, 56, day(2026, 11, 3));
    assert_eq!(under, [], "55 is under 56");
}

#[test]
fn a_certificate_is_raised_two_weeks_before_its_end_and_after_it() {
    let served = ServedCertificate {
        domain: "blog.example.com".to_owned(),
        not_after: day(2026, 12, 1),
    };
    let early = certificate(&served, day(2026, 11, 10));
    assert_eq!(early, [], "21 days left");
    let ending = certificate(&served, day(2026, 11, 20));
    assert_eq!(ending[0].key, "certificate:blog.example.com:2026-12-01");
    assert_eq!(
        ending[0].line,
        "The certificate for blog.example.com ends on 2026-12-01."
    );
    let ended = certificate(&served, day(2026, 12, 2));
    assert_eq!(
        ended[0].key,
        "certificate:blog.example.com:2026-12-01:ended"
    );
}

#[test]
fn posts_unchanged_too_long_are_stale_oldest_first_and_zero_days_is_off() {
    let post = |id: &str, changed| LivePost {
        id: id.to_owned(),
        title: format!("Post {id}"),
        changed,
    };
    let live = [
        post("en.fresh", day(2026, 10, 1)),
        post("en.older", day(2025, 1, 5)),
        post("en.oldest", day(2024, 3, 1)),
    ];
    let found = stale_posts(&live, 365, day(2026, 11, 3));
    let keys: Vec<&str> = found.iter().map(|f| f.key.as_str()).collect();
    assert_eq!(keys, ["stale-post:en.oldest", "stale-post:en.older"]);
    assert_eq!(
        found[0].line,
        "Post en.oldest (en.oldest), last changed 2024-03-01"
    );
    assert_eq!(stale_posts(&live, 0, day(2026, 11, 3)), []);
}
