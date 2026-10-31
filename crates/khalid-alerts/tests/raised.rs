//! Each problem is raised once while it lasts, and again after it clears
//! and comes back (KH-GRO-6).

use chrono::{DateTime, Duration, TimeZone, Utc};
use khalid_alerts::{Finding, Kind, MemoryRaised, Raised, fresh};

#[cfg(test)]
fn at() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 3, 9, 0, 0).unwrap()
}

#[cfg(test)]
fn finding(kind: Kind, subject: &str) -> Finding {
    Finding {
        key: kind.key(subject),
        line: subject.to_owned(),
    }
}

/// The subjects an alert raised, or none.
#[cfg(test)]
async fn raised(store: &MemoryRaised, kind: Kind, subjects: &[&str]) -> Vec<String> {
    let findings = subjects.iter().map(|s| finding(kind, s)).collect();
    let alert = fresh(store, kind, findings, at()).await.unwrap();
    alert.map_or_else(Vec::new, |alert| {
        alert.findings().iter().map(|f| f.line.clone()).collect()
    })
}

#[tokio::test]
async fn a_lasting_problem_is_raised_once_and_again_after_it_clears() {
    let store = MemoryRaised::default();
    let stale = Kind::StalePost;
    assert_eq!(
        raised(&store, stale, &["en.a", "en.b"]).await,
        ["en.a", "en.b"]
    );
    assert_eq!(
        raised(&store, stale, &["en.a", "en.b"]).await,
        Vec::<String>::new()
    );
    assert_eq!(
        raised(&store, stale, &["en.a"]).await,
        Vec::<String>::new(),
        "b cleared"
    );
    assert_eq!(
        raised(&store, stale, &["en.a", "en.b"]).await,
        ["en.b"],
        "b is back"
    );
}

#[tokio::test]
async fn a_one_off_problem_is_not_forgotten_when_the_next_round_misses_it() {
    let store = MemoryRaised::default();
    let refused = Kind::FailedSchedule;
    assert_eq!(raised(&store, refused, &["en.a"]).await, ["en.a"]);
    assert_eq!(raised(&store, refused, &["en.b"]).await, ["en.b"]);
    assert_eq!(
        raised(&store, refused, &["en.a"]).await,
        Vec::<String>::new()
    );
}

#[tokio::test]
async fn kinds_keep_their_own_keys_and_old_keys_are_dropped() {
    let store = MemoryRaised::default();
    raised(&store, Kind::StalePost, &["x"]).await;
    assert_eq!(
        raised(&store, Kind::Certificate, &[]).await,
        Vec::<String>::new()
    );
    assert_eq!(
        store.raised(Kind::StalePost).await.unwrap().len(),
        1,
        "kept"
    );
    store
        .forget_before(at() + Duration::days(30))
        .await
        .unwrap();
    assert_eq!(store.raised(Kind::StalePost).await.unwrap().len(), 0);
}
