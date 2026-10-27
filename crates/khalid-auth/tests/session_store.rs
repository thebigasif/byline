//! What every session store must do, shown on the one in memory. The
//! SQLite store in `khalid-db` runs the same cases.

use chrono::{DateTime, TimeDelta, TimeZone, Utc};
use khalid_auth::{AccountId, MemorySessions, SessionStore, SessionToken, StoredSession};

#[cfg(test)]
fn start() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0)
        .single()
        .expect("a time")
}

#[cfg(test)]
fn hash() -> khalid_auth::SessionHash {
    SessionToken::generate().expect("a random source").hash()
}

#[tokio::test]
async fn a_session_is_found_by_its_hash_and_remembers_when_it_was_used() {
    let store = MemorySessions::new();
    let (mine, unknown) = (hash(), hash());
    let ana = AccountId::from_row(1);
    store.begin(mine, ana, start()).await.expect("stored");
    assert_eq!(store.find(unknown).await.expect("read"), None);
    let later = start() + TimeDelta::hours(1);
    store.touch(mine, later).await.expect("written");
    let found = store.find(mine).await.expect("read");
    let want = StoredSession {
        account: ana,
        created_at: start(),
        last_seen_at: later,
    };
    assert_eq!(found, Some(want));
}

#[tokio::test]
async fn ending_one_session_or_all_of_an_account_keeps_the_rest() {
    let store = MemorySessions::new();
    let (ana, ben) = (AccountId::from_row(1), AccountId::from_row(2));
    let [one, two, three, bens] = [hash(), hash(), hash(), hash()];
    for (hash, account) in [(one, ana), (two, ana), (three, ana), (bens, ben)] {
        store.begin(hash, account, start()).await.expect("stored");
    }
    assert!(store.end(one).await.expect("written"));
    assert!(!store.end(one).await.expect("written"), "already ended");
    assert_eq!(store.end_all(ana, Some(two)).await.expect("written"), 1);
    assert!(
        store.find(two).await.expect("read").is_some(),
        "the kept one"
    );
    assert!(
        store.find(bens).await.expect("read").is_some(),
        "another account's"
    );
    assert_eq!(store.end_all(ana, None).await.expect("written"), 1);
    assert_eq!(store.len(), 1);
}
