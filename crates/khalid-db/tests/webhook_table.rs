//! The site's webhook list on SQLite: a new webhook comes back with the
//! secret it signs with, a switched-off one is left out, and each site
//! sees only its own (KH-API-3).

use std::path::PathBuf;

use chrono::{TimeZone, Utc};
use khalid_api::webhook_store::{StoredWebhook, WebhookStore};
use khalid_core::{AbsoluteUrl, FixedClock, Slug};
use khalid_db::{Database, WebhookTable};
use khalid_webhooks::{Event, SigningKey, Webhook, sign};
use sqlx::ConnectOptions;
use sqlx::sqlite::SqliteConnectOptions;

/// A migrated database with the sites `demo` and `other`, and the webhook
/// table of each.
#[cfg(test)]
async fn opened() -> (tempfile::TempDir, WebhookTable, WebhookTable) {
    let dir = tempfile::tempdir().unwrap();
    let mut path = PathBuf::from(dir.path());
    path.push("site.db");
    let db = Database::open(&path).await.unwrap();
    db.migrate().await.unwrap();
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap());
    let demo = db.site(&Slug::try_new("demo").unwrap()).await.unwrap();
    let other = db.site(&Slug::try_new("other").unwrap()).await.unwrap();
    let tables = (
        WebhookTable::new(db.clone(), demo, clock),
        WebhookTable::new(db, other, clock),
    );
    (dir, tables.0, tables.1)
}

#[cfg(test)]
fn hook() -> AbsoluteUrl {
    AbsoluteUrl::parse("https://hooks.example.com/khalid").unwrap()
}

#[tokio::test]
async fn a_new_webhook_is_listed_with_the_secret_it_signs_with() {
    let (_dir, demo, _) = opened().await;
    let (made, secret) = demo.create(&hook(), Event::PostPublished).await.unwrap();
    assert_eq!(secret.expose().len(), 64, "256 random bits as hex");

    let listed = demo.list().await.unwrap();
    assert_eq!(listed.len(), 1);
    let webhook = &listed[0];
    assert_eq!(webhook.id(), made.id());
    assert_eq!(webhook.url(), &hook());
    assert!(webhook.wants(Event::PostPublished));
    assert!(!webhook.wants(Event::PostDeleted));

    // The receiver's copy of the secret checks what the list signs.
    let at = Utc.with_ymd_and_hms(2026, 10, 27, 9, 5, 0).unwrap();
    let receiver = SigningKey::try_new(&secret).unwrap();
    assert_eq!(
        sign(webhook.key(), at, b"{}").to_string(),
        sign(&receiver, at, b"{}").to_string()
    );
}

#[tokio::test]
async fn two_webhooks_get_two_secrets() {
    let (_dir, demo, _) = opened().await;
    let (_, first) = demo.create(&hook(), Event::PostPublished).await.unwrap();
    let (_, second) = demo.create(&hook(), Event::PostUnpublished).await.unwrap();
    assert_ne!(first.expose(), second.expose());
    assert_eq!(demo.list().await.unwrap().len(), 2);
}

#[tokio::test]
async fn a_switched_off_webhook_is_left_out() {
    let (dir, demo, _) = opened().await;
    let (off, _) = demo.create(&hook(), Event::PostPublished).await.unwrap();
    let (on, _) = demo.create(&hook(), Event::PostScheduled).await.unwrap();
    let mut path = PathBuf::from(dir.path());
    path.push("site.db");
    let mut connection = SqliteConnectOptions::new()
        .filename(&path)
        .connect()
        .await
        .unwrap();
    sqlx::query("UPDATE webhooks SET disabled_at = '2026-10-27T09:10:00Z' WHERE id = ?1")
        .bind(off.id().get())
        .execute(&mut connection)
        .await
        .unwrap();
    let ids: Vec<_> = demo.list().await.unwrap().iter().map(Webhook::id).collect();
    assert_eq!(ids, [on.id()]);
}

#[tokio::test]
async fn each_site_lists_only_its_own_webhooks() {
    let (_dir, demo, other) = opened().await;
    demo.create(&hook(), Event::PostPublished).await.unwrap();
    assert!(other.list().await.unwrap().is_empty());
}

#[tokio::test]
async fn the_store_lists_adds_and_switches_off_webhooks_without_their_secrets() {
    let (_dir, demo, other) = opened().await;
    let store: &dyn WebhookStore = &demo;
    let (added, secret) = store.add(&hook(), Event::CommentAdded).await.unwrap();
    assert_eq!(secret.expose().len(), 64);
    let at = Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap();
    let want = StoredWebhook::new(added.id, Event::CommentAdded, hook(), at);
    assert_eq!(store.list().await.unwrap(), [want]);

    let theirs: &dyn WebhookStore = &other;
    assert!(
        !theirs.remove(added.id).await.unwrap(),
        "a site cannot switch off another site's webhook"
    );
    assert!(store.remove(added.id).await.unwrap());
    assert_eq!(store.list().await.unwrap(), Vec::new());
    assert!(
        demo.list().await.unwrap().is_empty(),
        "nothing is sent to it"
    );
    assert!(
        !store.remove(added.id).await.unwrap(),
        "already switched off"
    );
}
