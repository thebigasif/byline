//! Sending an issue, such as a post, to a segment against SQLite and the
//! fake transport: each confirmed subscriber gets one copy with their own
//! unsubscribe link and the postal address, a key goes out once, and a
//! refused copy is counted (task 5.8).

use std::path::PathBuf;
use std::sync::Arc;

use chrono::{DateTime, TimeZone, Utc};
use khalid_core::{AbsoluteUrl, FixedClock, Locale, Slug};
use khalid_db::{Database, SubscriberTable};
use khalid_newsletter::{
    EmailAddress, Issue, MemoryMailer, Newsletter, NewsletterSite, PostalAddress, Segment,
    SendTotals, Sent, SiteLinks, SubscriberStore, Unsubscribed,
};

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
    Utc.with_ymd_and_hms(2026, 10, 31, 9, 0, 0).unwrap()
}

#[cfg(test)]
fn newsletter(table: &SubscriberTable, mailer: &MemoryMailer) -> Newsletter {
    let address = AbsoluteUrl::parse("https://www.example.com").unwrap();
    let site = NewsletterSite {
        name: "Field Notes".to_owned(),
        locale: Locale::try_new("en").unwrap(),
        links: SiteLinks::new(&address, "/blog/"),
    };
    let (store, mail) = (Arc::new(table.clone()), Arc::new(mailer.clone()));
    Newsletter::new(store, mail, Arc::new(FixedClock::new(nine())), site).unwrap()
}

/// Confirmed subscribers of `main` at `addresses`, signed up through the
/// flow with a transport of their own.
#[cfg(test)]
async fn subscribers(table: &SubscriberTable, addresses: &[&str]) {
    let mailer = MemoryMailer::default();
    let flow = newsletter(table, &mailer);
    for address in addresses {
        let email = EmailAddress::parse(address).unwrap();
        flow.sign_up(email, main()).await.unwrap();
        let text = mailer.sent().last().unwrap().text.clone();
        let marker = "subscribe/confirm?token=";
        let start = text.find(marker).unwrap() + marker.len();
        flow.confirm(text.get(start..start + 64).unwrap())
            .await
            .unwrap();
    }
}

#[cfg(test)]
fn main() -> Slug {
    Slug::try_new("main").unwrap()
}

#[cfg(test)]
fn post() -> Issue {
    Issue::new(
        "post:en/anchor-text".to_owned(),
        "Choosing anchor text".to_owned(),
        "Choosing anchor text\n\nLinks read best when their words say where they go.\n".to_owned(),
    )
}

#[cfg(test)]
fn postal() -> PostalAddress {
    PostalAddress::new("Field Notes Ltd, 1 Park Row, Leeds LS1 5AB, UK").unwrap()
}

#[tokio::test]
async fn each_subscriber_gets_one_copy_with_their_own_link_and_the_postal_address() {
    let (_dir, table) = table().await;
    subscribers(&table, &["grace@example.com", "ada@example.com"]).await;
    let mailer = MemoryMailer::default();
    let flow = newsletter(&table, &mailer);
    let sent = flow
        .send(&post(), &Segment::new(main(), []), &postal())
        .await;
    let totals = SendTotals {
        sent: 2,
        ..SendTotals::default()
    };
    assert_eq!(sent.unwrap(), Sent::Now(totals));
    let copies = mailer.sent();
    let to: Vec<&str> = copies.iter().map(|mail| mail.to.as_str()).collect();
    assert_eq!(to, ["grace@example.com", "ada@example.com"]);
    let first = copies.first().unwrap();
    assert_eq!(first.subject, "Choosing anchor text");
    assert!(first.text.starts_with("Choosing anchor text\n\nLinks read"));
    assert!(
        first
            .text
            .ends_with("\n\nField Notes Ltd, 1 Park Row, Leeds LS1 5AB, UK\n")
    );
    let link = |text: &str| {
        let marker = "https://www.example.com/blog/unsubscribe?token=";
        let start = text.find(marker).unwrap() + marker.len();
        text.get(start..start + 64).unwrap().to_owned()
    };
    let (grace, ada) = (link(&first.text), link(&copies.last().unwrap().text));
    assert_ne!(grace, ada);
    assert_eq!(flow.unsubscribe(&grace).await.unwrap(), Unsubscribed::Done);
}

#[tokio::test]
async fn an_issue_goes_out_once_and_an_edit_never_sends_it_again() {
    let (_dir, table) = table().await;
    subscribers(&table, &["grace@example.com"]).await;
    let mailer = MemoryMailer::default();
    let flow = newsletter(&table, &mailer);
    let segment = Segment::new(main(), []);
    flow.send(&post(), &segment, &postal()).await.unwrap();
    let edited = Issue::new(
        "post:en/anchor-text".to_owned(),
        "Choosing anchor text, updated".to_owned(),
        "New words.\n".to_owned(),
    );
    let again = flow.send(&edited, &segment, &postal()).await.unwrap();
    assert_eq!(again, Sent::Before);
    assert_eq!(mailer.sent().len(), 1);
}

#[tokio::test]
async fn a_refused_copy_is_counted_and_the_key_stays_used() {
    let (_dir, table) = table().await;
    subscribers(&table, &["grace@example.com"]).await;
    let flow = newsletter(&table, &MemoryMailer::refusing());
    let segment = Segment::new(main(), []);
    let sent = flow.send(&post(), &segment, &postal()).await.unwrap();
    let totals = SendTotals {
        failed: 1,
        ..SendTotals::default()
    };
    assert_eq!(sent, Sent::Now(totals));
    let again = flow.send(&post(), &segment, &postal()).await.unwrap();
    assert_eq!(again, Sent::Before);
}

#[tokio::test]
async fn each_email_keeps_totals_and_counts_an_unsubscribe_once() {
    let (_dir, table) = table().await;
    subscribers(&table, &["grace@example.com", "ada@example.com"]).await;
    let mailer = MemoryMailer::default();
    let flow = newsletter(&table, &mailer);
    flow.send(&post(), &Segment::new(main(), []), &postal())
        .await
        .unwrap();
    let text = mailer.sent().first().unwrap().text.clone();
    let marker = "/blog/unsubscribe?token=";
    let start = text.find(marker).unwrap() + marker.len();
    let link = text.get(start..start + 64).unwrap();
    flow.unsubscribe(link).await.unwrap();
    flow.unsubscribe(link).await.unwrap();
    let numbers = table.sends().await.unwrap();
    assert_eq!(numbers.len(), 1, "{numbers:?}");
    let first = numbers.first().unwrap();
    assert_eq!(first.key, "post:en/anchor-text");
    assert_eq!(first.list, main());
    assert_eq!(first.started_at, nine());
    let totals = SendTotals { sent: 2, failed: 0 };
    assert_eq!((first.totals, first.unsubscribed), (totals, 1));
}
