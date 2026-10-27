//! Entries held in memory for tests: a new one is refused when its
//! identity is taken, and an edit or delete made on an older copy is
//! refused (BL-API-7).

use chrono::{DateTime, TimeZone, Utc};
use khalid_api::entry_store::{Entry, EntryError, EntryStore, MemoryEntries};
use khalid_api::resources::{AdminPage, NewPage, PageEdit};
use khalid_core::{FixedClock, Locale, Slug};
use serde_json::json;

#[cfg(test)]
fn at(day: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, day, 9, 0, 0).unwrap()
}

#[cfg(test)]
fn about() -> AdminPage {
    let key = (
        Locale::try_new("en").unwrap(),
        Slug::try_new("about").unwrap(),
    );
    AdminPage::new(key, "About Field Notes", "Who writes here.", at(20))
}

#[cfg(test)]
fn store() -> MemoryEntries<AdminPage> {
    MemoryEntries::new(vec![about()]).with_clock(FixedClock::new(at(27)))
}

#[tokio::test]
async fn a_new_page_is_kept_once_per_identity() {
    let store = store();
    let new: NewPage = serde_json::from_value(json!({
        "slug": "contact", "title": "Contact", "markdown": "Write to us.",
    }))
    .unwrap();
    let made = store.create(new.clone()).await.unwrap();
    assert_eq!(made.id(), "en.contact");
    assert_eq!(made.updated_at(), at(27));
    let again = store.create(new).await;
    assert!(
        matches!(again, Err(EntryError::Taken(ref id)) if id == "en.contact"),
        "{again:?}"
    );
    assert_eq!(store.list().await.unwrap().len(), 2);
}

#[tokio::test]
async fn an_edit_on_the_latest_copy_is_kept_and_stamped() {
    let store = store();
    let edit: PageEdit =
        serde_json::from_value(json!({"title": "About us", "updated_at": "2026-10-20T09:00:00Z"}))
            .unwrap();
    let page = store.update("en.about", edit).await.unwrap();
    assert_eq!(page.updated_at(), at(27));
    assert_eq!(store.get("en.about").await.unwrap(), Some(page));
}

#[tokio::test]
async fn an_edit_or_delete_on_an_older_copy_is_stale_and_changes_nothing() {
    let store = store();
    let edit: PageEdit =
        serde_json::from_value(json!({"title": "Lost", "updated_at": "2026-10-19T09:00:00Z"}))
            .unwrap();
    let stale = store.update("en.about", edit).await;
    assert!(
        matches!(stale, Err(EntryError::Stale { updated_at }) if updated_at == at(20)),
        "{stale:?}"
    );
    let stale = store.delete("en.about", at(19)).await;
    assert!(matches!(stale, Err(EntryError::Stale { .. })), "{stale:?}");
    assert_eq!(store.get("en.about").await.unwrap(), Some(about()));
}

#[tokio::test]
async fn a_delete_on_the_latest_copy_removes_the_page() {
    let store = store();
    store.delete("en.about", at(20)).await.unwrap();
    assert_eq!(store.get("en.about").await.unwrap(), None);
    let missing = store.delete("en.about", at(20)).await;
    assert!(matches!(missing, Err(EntryError::NotFound)), "{missing:?}");
}
