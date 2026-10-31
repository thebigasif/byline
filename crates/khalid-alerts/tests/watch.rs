//! The speed and uptime watch's checks (C6): a site that stopped answering
//! and a site that answers slowly.

use chrono::{DateTime, Duration, TimeZone, Utc};
use khalid_alerts::{Answer, Finding, Kind, Probe, site_down, slow_site};

const HOME: &str = "https://www.example.com/blog/";

#[cfg(test)]
fn at(minute: i64) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 3, 9, 0, 0).unwrap() + Duration::minutes(minute)
}

#[cfg(test)]
fn probe(minute: i64, answer: Answer, millis: u32) -> Probe {
    Probe {
        at: at(minute),
        answer,
        millis,
    }
}

#[cfg(test)]
fn ok(minute: i64, millis: u32) -> Probe {
    probe(minute, Answer::Status(200), millis)
}

#[test]
fn three_failed_looks_in_a_row_say_the_site_is_down_since_the_first() {
    let probes = [
        ok(0, 80),
        probe(5, Answer::Unreached, 10_000),
        probe(10, Answer::Status(503), 40),
        probe(15, Answer::Status(404), 30),
    ];
    assert_eq!(
        site_down(&probes, HOME),
        [Finding {
            key: "site-down:https://www.example.com/blog/".to_owned(),
            line: "https://www.example.com/blog/ has not answered since 09:05 UTC on 2026-11-03; the last look got status 404.".to_owned(),
        }]
    );
    assert_eq!(
        site_down(&probes[..3], HOME),
        [],
        "two in a row are not enough"
    );
    let unreached = [
        probe(5, Answer::Unreached, 10_000),
        probe(10, Answer::Unreached, 10_000),
        probe(15, Answer::Unreached, 10_000),
    ];
    let found = site_down(&unreached, HOME);
    assert!(found[0].line.ends_with("got no answer."), "{found:?}");
}

#[test]
fn a_redirect_or_a_page_is_up_and_one_good_look_ends_the_run() {
    let probes = [
        probe(0, Answer::Unreached, 10_000),
        probe(5, Answer::Unreached, 10_000),
        probe(10, Answer::Status(301), 20),
        probe(15, Answer::Unreached, 10_000),
    ];
    assert_eq!(site_down(&probes, HOME), []);
    assert_eq!(site_down(&[], HOME), []);
    assert!(Kind::SiteDown.clears(), "a site that answers again clears");
}

#[test]
fn the_middle_answer_time_of_the_last_hour_over_the_limit_is_slow() {
    let probes = [
        ok(-120, 5_000),
        ok(10, 900),
        ok(20, 1_500),
        ok(30, 1_400),
        probe(40, Answer::Unreached, 10_000),
        ok(50, 2_000),
    ];
    let found = slow_site(&probes, 1_000, HOME, at(55));
    assert_eq!(
        found,
        [Finding {
            key: "slow-site:https://www.example.com/blog/".to_owned(),
            line: "https://www.example.com/blog/ took 1500 ms in the middle of 4 answers in the last hour; the limit is 1000 ms.".to_owned(),
        }]
    );
    assert_eq!(slow_site(&probes, 1_500, HOME, at(55)), [], "not over");
    assert_eq!(slow_site(&probes, 0, HOME, at(55)), [], "0 is off");
    assert!(Kind::SlowSite.clears());
}

#[test]
fn fewer_than_three_answers_in_the_hour_say_nothing_about_speed() {
    let probes = [ok(10, 3_000), ok(20, 3_000)];
    assert_eq!(slow_site(&probes, 1_000, HOME, at(30)), []);
}

#[test]
fn the_new_kinds_have_wire_names_and_plain_words() {
    assert_eq!(Kind::SiteDown.as_str(), "site-down");
    assert_eq!(Kind::SlowSite.as_str(), "slow-site");
    let finding = Finding {
        key: Kind::SiteDown.key(HOME),
        line: "down".to_owned(),
    };
    let alert = khalid_alerts::Alert::new(Kind::SiteDown, vec![finding], at(0)).unwrap();
    assert_eq!(
        alert.subject("Field Notes"),
        "Field Notes: The blog did not answer at its public address"
    );
}
