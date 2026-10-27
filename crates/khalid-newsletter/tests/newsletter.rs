//! The sign-up flow against SQLite and the fake transport: one email per
//! sign-up, its links confirm or remove the address, and nothing is sent
//! that would fill an inbox or reveal who subscribes (task 5.5).

use std::path::PathBuf;
use std::sync::Arc;

use chrono::{DateTime, TimeDelta, TimeZone, Utc};
use khalid_core::{AbsoluteUrl, FixedClock, Locale, Slug};
use khalid_db::{Database, SubscriberTable};
use khalid_newsletter::{
    Confirmed, EmailAddress, MemoryMailer, Newsletter, NewsletterError, NewsletterSite, SignedUp,
    SiteLinks, Unsubscribed,
};

/// A site's subscribers in a fresh database, kept alive by the folder.
#[cfg(test)]
async fn table() -> (tempfile::TempDir, SubscriberTable) {
    let dir = tempfile::tempdir().unwrap();
    let mut path = PathBuf::from(dir.path());
    path.push("site.db");
    let db = Database::open(&path).await.unwrap();
    db.migrate().await.unwrap();
    let site = db.site(&Slug::try_new("demo").unwrap()).await.unwrap();
    (dir, SubscriberTable::new(db, site))
}

#[cfg(test)]
fn nine() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap()
}

/// The newsletter of "Field Notes" over `table`, sending through `mailer`,
/// with its clock stopped at `at`.
#[cfg(test)]
fn newsletter(table: &SubscriberTable, mailer: &MemoryMailer, at: DateTime<Utc>) -> Newsletter {
    let address = AbsoluteUrl::parse("https://www.example.com").unwrap();
    let site = NewsletterSite {
        name: "Field Notes".to_owned(),
        locale: Locale::try_new("en").unwrap(),
        links: SiteLinks::new(&address, "/blog/"),
    };
    let (store, mail) = (Arc::new(table.clone()), Arc::new(mailer.clone()));
    Newsletter::new(store, mail, Arc::new(FixedClock::new(at)), site).unwrap()
}

#[cfg(test)]
fn grace() -> EmailAddress {
    EmailAddress::parse("grace@example.com").unwrap()
}

#[cfg(test)]
fn main_list() -> Slug {
    Slug::try_new("main").unwrap()
}

/// The token after `page?token=` in the one email `mailer` sent.
#[cfg(test)]
fn token_in(mailer: &MemoryMailer, page: &str) -> String {
    let sent = mailer.sent();
    assert_eq!(sent.len(), 1, "{sent:?}");
    let text = &sent.first().unwrap().text;
    let marker = format!("/blog/{page}?token=");
    let start = text.find(&marker).unwrap() + marker.len();
    text.get(start..start + 64).unwrap().to_owned()
}

#[tokio::test]
async fn a_sign_up_sends_one_email_whose_link_confirms() {
    let (_dir, table) = table().await;
    let mailer = MemoryMailer::default();
    let flow = newsletter(&table, &mailer, nine());
    let signed = flow.sign_up(grace(), main_list()).await.unwrap();
    assert!(matches!(signed, SignedUp::ConfirmationDue(_)), "{signed:?}");
    assert_eq!(mailer.sent().first().unwrap().to, grace());
    let token = token_in(&mailer, "subscribe/confirm");
    let later = newsletter(&table, &mailer, nine() + TimeDelta::hours(1));
    assert!(matches!(
        later.confirm(&token).await.unwrap(),
        Confirmed::Now(_)
    ));
    assert_eq!(later.confirm(&token).await.unwrap(), Confirmed::Before);
}

#[tokio::test]
async fn the_link_stops_working_after_two_days() {
    let (_dir, table) = table().await;
    let mailer = MemoryMailer::default();
    newsletter(&table, &mailer, nine())
        .sign_up(grace(), main_list())
        .await
        .unwrap();
    let token = token_in(&mailer, "subscribe/confirm");
    let late = newsletter(&table, &mailer, nine() + TimeDelta::days(3));
    assert_eq!(late.confirm(&token).await.unwrap(), Confirmed::Expired);
}

#[tokio::test]
async fn the_remove_link_in_the_email_unsubscribes_at_once() {
    let (_dir, table) = table().await;
    let mailer = MemoryMailer::default();
    let flow = newsletter(&table, &mailer, nine());
    flow.sign_up(grace(), main_list()).await.unwrap();
    let remove = token_in(&mailer, "unsubscribe");
    let confirm = token_in(&mailer, "subscribe/confirm");
    assert_eq!(flow.unsubscribe(&remove).await.unwrap(), Unsubscribed::Done);
    assert_eq!(flow.confirm(&confirm).await.unwrap(), Confirmed::Unknown);
}

#[tokio::test]
async fn forms_sent_again_and_again_send_one_email() {
    let (_dir, table) = table().await;
    let mailer = MemoryMailer::default();
    let flow = newsletter(&table, &mailer, nine());
    flow.sign_up(grace(), main_list()).await.unwrap();
    for _ in 0..5 {
        let again = flow.sign_up(grace(), main_list()).await.unwrap();
        assert_eq!(again, SignedUp::RecentlyAsked);
    }
    assert_eq!(mailer.sent().len(), 1);
}

#[tokio::test]
async fn a_confirmed_reader_signing_up_again_gets_no_email() {
    let (_dir, table) = table().await;
    let mailer = MemoryMailer::default();
    let flow = newsletter(&table, &mailer, nine());
    flow.sign_up(grace(), main_list()).await.unwrap();
    flow.confirm(&token_in(&mailer, "subscribe/confirm"))
        .await
        .unwrap();
    let again = flow.sign_up(grace(), main_list()).await.unwrap();
    assert_eq!(again, SignedUp::AlreadyConfirmed);
    assert_eq!(mailer.sent().len(), 1);
}

#[tokio::test]
async fn text_that_is_not_a_token_does_nothing() {
    let (_dir, table) = table().await;
    let flow = newsletter(&table, &MemoryMailer::default(), nine());
    for text in ["", "not-a-token", "'; DROP TABLE subscribers; --"] {
        assert_eq!(flow.confirm(text).await.unwrap(), Confirmed::Unknown);
        assert_eq!(flow.unsubscribe(text).await.unwrap(), Unsubscribed::Unknown);
    }
}

#[tokio::test]
async fn a_transport_that_is_down_is_an_error() {
    let (_dir, table) = table().await;
    let flow = newsletter(&table, &MemoryMailer::refusing(), nine());
    let error = flow.sign_up(grace(), main_list()).await.unwrap_err();
    assert!(matches!(error, NewsletterError::Mail(_)), "{error:?}");
}
