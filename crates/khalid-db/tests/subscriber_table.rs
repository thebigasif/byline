//! Subscribers on SQLite: an address waits for its confirmation, a link
//! works once and expires, a second email waits ten minutes, one click
//! unsubscribes from everything, the file holds hashes only, and each site
//! sees only its own subscribers (task 5.5, RULES.md BL-SEC-3).

use std::path::PathBuf;

use chrono::{DateTime, TimeDelta, TimeZone, Utc};
use khalid_core::Slug;
use khalid_db::{Database, SubscriberTable};
use khalid_newsletter::{
    Confirmed, EmailAddress, SignUp, SignedUp, SubscriberId, SubscriberStore, SubscriberToken,
    Unsubscribed,
};
use sqlx::SqlitePool;

/// The tables of the sites `demo` and `other` in a migrated database, and
/// a connection to look at the file with.
struct Opened {
    _dir: tempfile::TempDir,
    file: SqlitePool,
    demo: SubscriberTable,
    other: SubscriberTable,
}

#[cfg(test)]
async fn opened() -> Opened {
    let dir = tempfile::tempdir().unwrap();
    let mut path = PathBuf::from(dir.path());
    path.push("site.db");
    let db = Database::open(&path).await.unwrap();
    db.migrate().await.unwrap();
    let demo = db.site(&Slug::try_new("demo").unwrap()).await.unwrap();
    let other = db.site(&Slug::try_new("other").unwrap()).await.unwrap();
    let url = format!("sqlite://{}", path.display());
    Opened {
        _dir: dir,
        file: SqlitePool::connect(&url).await.unwrap(),
        demo: SubscriberTable::new(db.clone(), demo),
        other: SubscriberTable::new(db, other),
    }
}

#[cfg(test)]
fn nine() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap()
}

/// A sign-up of `email` to `list` at `at`, and its confirmation token.
#[cfg(test)]
fn sign_up(email: &str, list: &str, at: DateTime<Utc>) -> (SignUp, SubscriberToken) {
    let token = SubscriberToken::generate().unwrap();
    let email = EmailAddress::parse(email).unwrap();
    let list = Slug::try_new(list).unwrap();
    (SignUp::new(email, list, token.hash(), at), token)
}

/// Records `asked` in `table`, which must make a confirmation due.
#[cfg(test)]
async fn due(table: &SubscriberTable, asked: &SignUp) -> SubscriberId {
    match table.sign_up(asked).await.unwrap() {
        SignedUp::ConfirmationDue(id) => id,
        other => panic!("no confirmation was due: {other:?}"),
    }
}

#[cfg(test)]
async fn open(table: &SubscriberTable, token: &SubscriberToken, at: DateTime<Utc>) -> Confirmed {
    table.confirm(&token.hash(), at).await.unwrap()
}

#[cfg(test)]
async fn status(file: &SqlitePool) -> String {
    sqlx::query_scalar("SELECT status FROM subscribers")
        .fetch_one(file)
        .await
        .unwrap()
}

#[tokio::test]
async fn an_address_waits_until_its_link_is_opened_once() {
    let db = opened().await;
    let (asked, token) = sign_up("grace@example.com", "main", nine());
    let id = due(&db.demo, &asked).await;
    assert_eq!(status(&db.file).await, "pending");
    let later = nine() + TimeDelta::hours(3);
    assert_eq!(open(&db.demo, &token, later).await, Confirmed::Now(id));
    assert_eq!(status(&db.file).await, "confirmed");
    assert_eq!(open(&db.demo, &token, later).await, Confirmed::Before);
}

#[tokio::test]
async fn a_link_older_than_two_days_confirms_nobody() {
    let db = opened().await;
    let (asked, token) = sign_up("grace@example.com", "main", nine());
    due(&db.demo, &asked).await;
    let late = nine() + TimeDelta::days(2);
    assert_eq!(open(&db.demo, &token, late).await, Confirmed::Expired);
    assert_eq!(status(&db.file).await, "pending");
}

#[tokio::test]
async fn a_second_form_within_ten_minutes_sends_nothing_and_later_one_more() {
    let db = opened().await;
    let (first, _) = sign_up("grace@example.com", "main", nine());
    due(&db.demo, &first).await;
    let soon = nine() + TimeDelta::minutes(9);
    let (again, unsent) = sign_up("GRACE@example.com", "main", soon);
    let answer = db.demo.sign_up(&again).await.unwrap();
    assert_eq!(answer, SignedUp::RecentlyAsked);
    assert_eq!(open(&db.demo, &unsent, soon).await, Confirmed::Unknown);
    let later = nine() + TimeDelta::minutes(11);
    let (third, token) = sign_up("grace@example.com", "main", later);
    let id = due(&db.demo, &third).await;
    assert_eq!(open(&db.demo, &token, later).await, Confirmed::Now(id));
}

#[tokio::test]
async fn a_confirmed_address_joins_another_newsletter_without_an_email() {
    let db = opened().await;
    let (asked, token) = sign_up("grace@example.com", "main", nine());
    due(&db.demo, &asked).await;
    open(&db.demo, &token, nine()).await;
    let (more, _) = sign_up("grace@example.com", "releases", nine());
    let answer = db.demo.sign_up(&more).await.unwrap();
    assert_eq!(answer, SignedUp::AlreadyConfirmed);
    let lists: Vec<String> = sqlx::query_scalar("SELECT list FROM subscriber_lists ORDER BY list")
        .fetch_all(&db.file)
        .await
        .unwrap();
    assert_eq!(lists, ["main", "releases"]);
}

#[tokio::test]
async fn a_token_that_was_never_sent_confirms_nobody() {
    let db = opened().await;
    let stranger = SubscriberToken::generate().unwrap();
    assert_eq!(open(&db.demo, &stranger, nine()).await, Confirmed::Unknown);
}

#[tokio::test]
async fn the_file_holds_the_hash_and_never_the_token() {
    let db = opened().await;
    let (asked, token) = sign_up("grace@example.com", "main", nine());
    due(&db.demo, &asked).await;
    let stored: Vec<Vec<u8>> = sqlx::query_scalar("SELECT token_hash FROM subscriber_tokens")
        .fetch_all(&db.file)
        .await
        .unwrap();
    assert_eq!(stored, [token.hash().as_bytes().to_vec()]);
}

#[tokio::test]
async fn a_site_sees_only_its_own_subscribers() {
    let db = opened().await;
    let (asked, token) = sign_up("grace@example.com", "main", nine());
    due(&db.demo, &asked).await;
    assert_eq!(open(&db.other, &token, nine()).await, Confirmed::Unknown);
    let (same, _) = sign_up("grace@example.com", "main", nine());
    due(&db.other, &same).await;
}

/// Gives subscriber `id` of `table` a new unsubscribe link, as a send would.
#[cfg(test)]
async fn leave_link(table: &SubscriberTable, id: SubscriberId) -> SubscriberToken {
    let token = SubscriberToken::generate().unwrap();
    table
        .add_unsubscribe(id, &token.hash(), nine())
        .await
        .unwrap();
    token
}

#[cfg(test)]
async fn leave(
    table: &SubscriberTable,
    token: &SubscriberToken,
    at: DateTime<Utc>,
) -> Unsubscribed {
    table.unsubscribe(&token.hash(), at).await.unwrap()
}

#[tokio::test]
async fn one_click_unsubscribes_from_everything_and_a_second_click_changes_nothing() {
    let db = opened().await;
    let (asked, token) = sign_up("grace@example.com", "main", nine());
    let id = due(&db.demo, &asked).await;
    open(&db.demo, &token, nine()).await;
    let (more, _) = sign_up("grace@example.com", "releases", nine());
    db.demo.sign_up(&more).await.unwrap();
    let link = leave_link(&db.demo, id).await;
    let much_later = nine() + TimeDelta::days(400);
    assert_eq!(leave(&db.demo, &link, much_later).await, Unsubscribed::Done);
    assert_eq!(status(&db.file).await, "unsubscribed");
    let lists: i64 = sqlx::query_scalar("SELECT count(*) FROM subscriber_lists")
        .fetch_one(&db.file)
        .await
        .unwrap();
    assert_eq!(lists, 0);
    assert_eq!(leave(&db.demo, &link, much_later).await, Unsubscribed::Done);
}

#[tokio::test]
async fn after_unsubscribing_an_old_link_confirms_nobody_and_a_new_sign_up_asks_again() {
    let db = opened().await;
    let (first, old) = sign_up("grace@example.com", "main", nine());
    let id = due(&db.demo, &first).await;
    let link = leave_link(&db.demo, id).await;
    leave(&db.demo, &link, nine()).await;
    assert_eq!(open(&db.demo, &old, nine()).await, Confirmed::Unknown);
    let (back, token) = sign_up("grace@example.com", "main", nine());
    assert_eq!(due(&db.demo, &back).await, id);
    assert_eq!(status(&db.file).await, "pending");
    assert_eq!(open(&db.demo, &token, nine()).await, Confirmed::Now(id));
}

#[tokio::test]
async fn a_link_that_was_never_sent_unsubscribes_nobody() {
    let db = opened().await;
    let (asked, token) = sign_up("grace@example.com", "main", nine());
    due(&db.demo, &asked).await;
    // A confirmation token is not an unsubscribe link.
    assert_eq!(leave(&db.demo, &token, nine()).await, Unsubscribed::Unknown);
    let stranger = SubscriberToken::generate().unwrap();
    assert_eq!(
        leave(&db.demo, &stranger, nine()).await,
        Unsubscribed::Unknown
    );
    assert_eq!(status(&db.file).await, "pending");
}

#[tokio::test]
async fn a_site_cannot_unsubscribe_or_link_another_site_s_subscriber() {
    let db = opened().await;
    let (asked, _) = sign_up("grace@example.com", "main", nine());
    let id = due(&db.demo, &asked).await;
    let link = leave_link(&db.demo, id).await;
    assert_eq!(leave(&db.other, &link, nine()).await, Unsubscribed::Unknown);
    let token = SubscriberToken::generate().unwrap();
    let added = db.other.add_unsubscribe(id, &token.hash(), nine()).await;
    assert!(added.is_err(), "{added:?}");
    assert_eq!(status(&db.file).await, "pending");
}
