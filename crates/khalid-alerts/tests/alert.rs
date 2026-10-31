//! An alert and the words an owner reads (KH-GRO-6).

use chrono::{DateTime, TimeZone, Utc};
use khalid_alerts::{Alert, Finding, Kind};

#[cfg(test)]
fn day(year: i32, month: u32, day: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(year, month, day, 9, 30, 0).unwrap()
}

#[test]
fn an_alert_says_what_why_and_fix_then_each_finding() {
    let finding = Finding {
        key: Kind::StalePost.key("en.oldest"),
        line: "Oldest (en.oldest), last changed 2024-03-01".to_owned(),
    };
    assert_eq!(
        Alert::new(Kind::StalePost, Vec::new(), day(2026, 11, 3)),
        None
    );
    let alert = Alert::new(Kind::StalePost, vec![finding], day(2026, 11, 3)).unwrap();
    assert_eq!(
        alert.subject("Field Notes"),
        "Field Notes: Some live posts have not changed for a long time"
    );
    let text = alert.text();
    assert!(
        text.ends_with("save it.\n\n- Oldest (en.oldest), last changed 2024-03-01\n"),
        "{text}"
    );
    let json = serde_json::to_value(&alert).unwrap();
    assert_eq!(json["kind"], "stale-post");
    assert_eq!(json["raised_at"], "2026-11-03T09:30:00Z");
    assert_eq!(json["findings"][0]["key"], "stale-post:en.oldest");
}

#[test]
fn every_kind_owns_only_its_keys_and_says_each_thing_in_one_sentence() {
    for kind in Kind::ALL {
        assert!(kind.owns(&kind.key("x")));
        assert!(!kind.owns(&format!("{}x:y", kind.as_str())));
        let finding = Finding {
            key: kind.key("x"),
            line: "x".to_owned(),
        };
        let alert = Alert::new(kind, vec![finding], day(2026, 1, 1)).unwrap();
        let json = serde_json::to_value(&alert).unwrap();
        for part in ["what", "why", "fix"] {
            let sentence = json[part].as_str().unwrap();
            assert!(sentence.ends_with('.'), "{kind:?} {part}");
            let inner = sentence.matches(". ").count();
            assert_eq!(inner, 0, "{kind:?} {part} is one sentence");
            assert!(!sentence.contains('\u{2014}'), "no em dash");
        }
    }
}
