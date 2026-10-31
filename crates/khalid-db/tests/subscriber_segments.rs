//! Segments and labels on SQLite: a segment is one newsletter's confirmed
//! subscribers who carry every label asked for, oldest first, and each
//! site sees only its own (G39).

use std::path::PathBuf;

use chrono::{DateTime, TimeZone, Utc};
use khalid_core::Slug;
use khalid_db::{Database, SubscriberTable};
use khalid_newsletter::{
    Confirmed, EmailAddress, Labelled, Segment, SignUp, SignedUp, SubscriberStore, SubscriberToken,
};

#[cfg(test)]
async fn tables() -> (tempfile::TempDir, SubscriberTable, SubscriberTable) {
    let dir = tempfile::tempdir().unwrap();
    let mut path = PathBuf::from(dir.path());
    path.push("site.db");
    let db = Database::open(&path).await.unwrap();
    db.migrate().await.unwrap();
    let demo = db.site(&Slug::try_new("demo").unwrap()).await.unwrap();
    let other = db.site(&Slug::try_new("other").unwrap()).await.unwrap();
    let (demo, other) = (
        SubscriberTable::new(db.clone(), demo),
        SubscriberTable::new(db, other),
    );
    (dir, demo, other)
}

#[cfg(test)]
fn nine() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 31, 9, 0, 0).unwrap()
}

#[cfg(test)]
fn slug(text: &str) -> Slug {
    Slug::try_new(text).unwrap()
}

#[cfg(test)]
fn email(text: &str) -> EmailAddress {
    EmailAddress::parse(text).unwrap()
}

/// What happens after a sign-up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Then {
    Confirm,
    Wait,
}

/// Signs `address` up for `list`, then confirms it or leaves it waiting.
#[cfg(test)]
async fn subscribe(table: &SubscriberTable, address: &str, list: &str, then: Then) {
    let token = SubscriberToken::generate().unwrap();
    let asked = SignUp::new(email(address), slug(list), token.hash(), nine());
    let signed = table.sign_up(&asked).await.unwrap();
    if then == Then::Confirm && matches!(signed, SignedUp::ConfirmationDue(_)) {
        let done = table.confirm(&token.hash(), nine()).await.unwrap();
        assert!(matches!(done, Confirmed::Now(_)), "{done:?}");
    }
}

#[cfg(test)]
async fn audience(table: &SubscriberTable, list: &str, labels: &[&str]) -> Vec<String> {
    let segment = Segment::new(slug(list), labels.iter().map(|label| slug(label)));
    let found = table.audience(&segment).await.unwrap();
    found.iter().map(|r| r.email.as_str().to_owned()).collect()
}

#[tokio::test]
async fn a_segment_is_the_confirmed_members_of_one_newsletter() {
    let (_dir, demo, _) = tables().await;
    subscribe(&demo, "grace@example.com", "main", Then::Confirm).await;
    subscribe(&demo, "ada@example.com", "weekly", Then::Confirm).await;
    subscribe(&demo, "alan@example.com", "main", Then::Wait).await;
    subscribe(&demo, "ada@example.com", "main", Then::Confirm).await;
    assert_eq!(
        audience(&demo, "main", &[]).await,
        ["grace@example.com", "ada@example.com"]
    );
    assert_eq!(audience(&demo, "weekly", &[]).await, ["ada@example.com"]);
}

#[tokio::test]
async fn labels_narrow_a_segment_to_who_carries_every_one() {
    let (_dir, demo, _) = tables().await;
    for address in ["grace@example.com", "ada@example.com", "alan@example.com"] {
        subscribe(&demo, address, "main", Then::Confirm).await;
    }
    let paid = slug("paid");
    for address in ["grace@example.com", "ada@example.com"] {
        let done = demo.label(&email(address), &paid, nine()).await.unwrap();
        assert_eq!(done, Labelled::Done);
    }
    let early = slug("early");
    demo.label(&email("ada@example.com"), &early, nine())
        .await
        .unwrap();
    assert_eq!(
        audience(&demo, "main", &["paid"]).await,
        ["grace@example.com", "ada@example.com"]
    );
    assert_eq!(
        audience(&demo, "main", &["paid", "early"]).await,
        ["ada@example.com"]
    );
    let again = demo.label(&email("ada@example.com"), &paid, nine()).await;
    assert_eq!(again.unwrap(), Labelled::Done);
    demo.unlabel(&email("ada@example.com"), &paid)
        .await
        .unwrap();
    assert_eq!(
        audience(&demo, "main", &["paid"]).await,
        ["grace@example.com"]
    );
}

#[tokio::test]
async fn only_a_subscriber_of_this_site_can_be_labelled() {
    let (_dir, demo, other) = tables().await;
    subscribe(&demo, "grace@example.com", "main", Then::Confirm).await;
    let paid = slug("paid");
    let stranger = demo.label(&email("ada@example.com"), &paid, nine()).await;
    assert_eq!(stranger.unwrap(), Labelled::NotSubscribed);
    let across = other
        .label(&email("grace@example.com"), &paid, nine())
        .await;
    assert_eq!(across.unwrap(), Labelled::NotSubscribed);
    assert_eq!(
        audience(&demo, "main", &["paid"]).await,
        Vec::<String>::new()
    );
    assert_eq!(audience(&other, "main", &[]).await, Vec::<String>::new());
}
