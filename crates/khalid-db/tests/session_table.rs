//! Sessions on SQLite keep the contract `khalid-auth` tests on its
//! in-memory store, store only the token's hash, and end with their
//! account.

use std::path::PathBuf;

use chrono::{DateTime, TimeDelta, TimeZone, Utc};
use khalid_api::keys::Role;
use khalid_auth::{Account, AccountStore, SessionHash, SessionStore, SessionToken, StoredSession};
use khalid_core::Slug;
use khalid_db::{AccountTable, Database, SessionTable};

#[cfg(test)]
fn at() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap()
}

#[cfg(test)]
fn hash() -> SessionHash {
    SessionToken::generate().unwrap().hash()
}

/// A migrated database with the sites `demo` and `other`, the sessions of
/// each, the accounts of `demo`, and Asif and Mia on `demo`.
#[cfg(test)]
async fn opened() -> (
    tempfile::TempDir,
    SessionTable,
    SessionTable,
    AccountTable,
    [Account; 2],
) {
    let dir = tempfile::tempdir().unwrap();
    let mut path = PathBuf::from(dir.path());
    path.push("site.db");
    let db = Database::open(&path).await.unwrap();
    db.migrate().await.unwrap();
    let demo = db.site(&Slug::try_new("demo").unwrap()).await.unwrap();
    let other = db.site(&Slug::try_new("other").unwrap()).await.unwrap();
    let accounts = AccountTable::new(db.clone(), demo);
    let mut people = Vec::new();
    for (email, name) in [("asif@example.com", "Asif"), ("mia@example.com", "Mia")] {
        let email = email.parse().unwrap();
        people.push(
            accounts
                .create(&email, name, Role::Editor, at())
                .await
                .unwrap(),
        );
    }
    let people: [Account; 2] = people.try_into().unwrap();
    let sessions = SessionTable::new(db.clone(), demo);
    (
        dir,
        sessions,
        SessionTable::new(db, other),
        accounts,
        people,
    )
}

#[tokio::test]
async fn a_session_reads_back_and_remembers_when_it_was_used() {
    let (_dir, sessions, other, _, [asif, _]) = opened().await;
    let mine = hash();
    sessions.begin(mine, asif.id(), at()).await.unwrap();
    assert_eq!(sessions.find(hash()).await.unwrap(), None);
    assert_eq!(
        other.find(mine).await.unwrap(),
        None,
        "another site's session"
    );
    let later = at() + TimeDelta::hours(1);
    sessions.touch(mine, later).await.unwrap();
    let want = StoredSession {
        account: asif.id(),
        created_at: at(),
        last_seen_at: later,
    };
    assert_eq!(sessions.find(mine).await.unwrap(), Some(want));
}

#[tokio::test]
async fn ending_one_session_or_all_of_an_account_keeps_the_rest() {
    let (_dir, sessions, _, _, [asif, mia]) = opened().await;
    let [one, two, three, hers] = [hash(), hash(), hash(), hash()];
    for (hash, account) in [(one, &asif), (two, &asif), (three, &asif), (hers, &mia)] {
        sessions.begin(hash, account.id(), at()).await.unwrap();
    }
    assert!(sessions.end(one).await.unwrap());
    assert!(!sessions.end(one).await.unwrap(), "already ended");
    assert_eq!(sessions.end_all(asif.id(), Some(two)).await.unwrap(), 1);
    assert!(sessions.find(two).await.unwrap().is_some(), "the kept one");
    assert!(
        sessions.find(hers).await.unwrap().is_some(),
        "another account's"
    );
    assert_eq!(sessions.end_all(asif.id(), None).await.unwrap(), 1);
}

#[tokio::test]
async fn removing_an_account_ends_its_sessions() {
    let (_dir, sessions, _, accounts, [asif, _]) = opened().await;
    let mine = hash();
    sessions.begin(mine, asif.id(), at()).await.unwrap();
    accounts.remove(asif.id()).await.unwrap();
    assert_eq!(sessions.find(mine).await.unwrap(), None);
}
