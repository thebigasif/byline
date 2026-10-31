//! Welcome emails against SQLite and the fake transport: a reader gets the
//! welcome of each newsletter they joined the moment they confirm, once,
//! with an unsubscribe link and the postal address (task 5.9).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use chrono::{TimeZone, Utc};
use khalid_core::{AbsoluteUrl, FixedClock, Locale, Slug};
use khalid_db::{Database, SubscriberTable};
use khalid_newsletter::{
    EmailAddress, Lists, MemoryMailer, Newsletter, NewsletterSite, PostalAddress, SiteLinks,
    Welcome, Welcomes,
};

const WELCOME: &str =
    "# Welcome to the weekly links\n\nEvery Friday, five links worth your time.\n";

#[cfg(test)]
fn slug(text: &str) -> Slug {
    Slug::try_new(text).unwrap()
}

/// A newsletter whose `weekly` list has a welcome, over a fresh database.
#[cfg(test)]
async fn newsletter(mailer: &MemoryMailer) -> (tempfile::TempDir, Newsletter) {
    let dir = tempfile::tempdir().unwrap();
    let mut path = PathBuf::from(dir.path());
    path.push("site.db");
    let db = Database::open(&path).await.unwrap();
    db.migrate().await.unwrap();
    let id = db.site(&slug("demo")).await.unwrap();
    let address = AbsoluteUrl::parse("https://www.example.com").unwrap();
    let site = NewsletterSite {
        name: "Field Notes".to_owned(),
        locale: Locale::try_new("en").unwrap(),
        links: SiteLinks::new(&address, "/blog/"),
    };
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 10, 31, 9, 0, 0).unwrap());
    let store = Arc::new(SubscriberTable::new(db, id));
    let flow = Newsletter::new(store, Arc::new(mailer.clone()), Arc::new(clock), site).unwrap();
    let welcomes = BTreeMap::from([(slug("weekly"), Welcome::read(WELCOME).unwrap())]);
    let postal = PostalAddress::new("Field Notes Ltd, 1 Park Row, Leeds LS1 5AB, UK").unwrap();
    let flow = flow
        .with_lists(Lists::new([slug("weekly")]))
        .with_welcomes(Welcomes::new(welcomes, postal));
    (dir, flow)
}

/// Signs `address` up for `list` and opens the confirmation link.
#[cfg(test)]
async fn join(flow: &Newsletter, mailer: &MemoryMailer, address: &str, list: &str) {
    let email = EmailAddress::parse(address).unwrap();
    flow.sign_up(email, slug(list)).await.unwrap();
    let text = mailer.sent().last().unwrap().text.clone();
    let marker = "subscribe/confirm?token=";
    let start = text.find(marker).unwrap() + marker.len();
    let token = text.get(start..start + 64).unwrap().to_owned();
    flow.confirm(&token).await.unwrap();
    flow.confirm(&token).await.unwrap();
}

#[test]
fn a_welcome_is_a_heading_for_the_subject_and_text_below() {
    let welcome = Welcome::read(WELCOME).unwrap();
    assert_eq!(welcome.subject(), "Welcome to the weekly links");
    assert_eq!(
        welcome.text(),
        "Every Friday, five links worth your time.\n"
    );
    assert_eq!(Welcome::read("Welcome\n\nText.\n"), None);
    assert_eq!(Welcome::read("# Welcome\n"), None);
}

#[tokio::test]
async fn confirming_sends_the_welcome_of_the_newsletter_joined_once() {
    let mailer = MemoryMailer::default();
    let (_dir, flow) = newsletter(&mailer).await;
    join(&flow, &mailer, "grace@example.com", "weekly").await;
    let sent = mailer.sent();
    assert_eq!(sent.len(), 2, "{sent:?}");
    let welcome = sent.last().unwrap();
    assert_eq!(welcome.to.as_str(), "grace@example.com");
    assert_eq!(welcome.subject, "Welcome to the weekly links");
    assert!(welcome.text.starts_with("Every Friday"), "{}", welcome.text);
    assert!(
        welcome
            .text
            .contains("https://www.example.com/blog/unsubscribe?token=")
    );
    assert!(
        welcome.text.ends_with("Leeds LS1 5AB, UK\n"),
        "{}",
        welcome.text
    );
}

#[tokio::test]
async fn a_newsletter_without_a_welcome_sends_none() {
    let mailer = MemoryMailer::default();
    let (_dir, flow) = newsletter(&mailer).await;
    join(&flow, &mailer, "grace@example.com", "main").await;
    assert_eq!(mailer.sent().len(), 1, "only the confirmation");
}
