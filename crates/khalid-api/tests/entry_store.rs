//! The site's pages, snippets and authors as the Admin API keeps them
//! (KH-API-2).

use chrono::{DateTime, TimeZone, Utc};
use khalid_api::entry_store::Entry;
use khalid_api::resources::{AdminPage, NewPage, PageEdit};
use khalid_core::{Locale, Slug};
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
        .with_description(Some("Who writes Field Notes."))
}

#[test]
fn a_page_is_sent_in_api_shape() {
    assert_eq!(
        serde_json::to_value(about()).unwrap(),
        json!({
            "id": "en.about",
            "slug": "about",
            "language": "en",
            "title": "About Field Notes",
            "description": "Who writes Field Notes.",
            "markdown": "Who writes here.",
            "updated_at": "2026-10-20T09:00:00Z",
        })
    );
}

#[test]
fn a_new_page_is_made_in_its_language_and_an_edit_changes_only_what_it_sends() {
    let new: NewPage = serde_json::from_value(json!({
        "language": "de", "slug": "kontakt", "title": "Kontakt", "markdown": "Schreib uns.",
    }))
    .unwrap();
    let english = Locale::try_new("en").unwrap();
    let language = AdminPage::language(&new).cloned().unwrap_or(english);
    let made = AdminPage::made(new, language, at(27));
    assert_eq!(made.id(), "de.kontakt");
    assert_eq!(made.updated_at(), at(27));

    let edit: PageEdit = serde_json::from_value(json!({
        "title": "About us", "description": "", "updated_at": "2026-10-20T09:00:00Z",
    }))
    .unwrap();
    assert_eq!(AdminPage::seen(&edit), at(20));
    let page = serde_json::to_value(about().edited(edit, at(27))).unwrap();
    assert_eq!(page["title"], "About us");
    assert_eq!(page["description"], json!(null), "an empty one removes it");
    assert_eq!(page["markdown"], "Who writes here.");
    assert_eq!(page["updated_at"], "2026-10-27T09:00:00Z");
}

#[test]
fn a_new_page_with_a_slug_that_is_not_one_is_refused() {
    let new = serde_json::from_value::<NewPage>(json!({
        "slug": "About Us!", "title": "About", "markdown": "Text.",
    }));
    assert!(new.is_err(), "{new:?}");
}
