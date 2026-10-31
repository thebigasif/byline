//! The site's pages, snippets and authors as the Admin API keeps them
//! (KH-API-2).

use chrono::{DateTime, TimeZone, Utc};
use khalid_api::entry_store::Entry;
use khalid_api::resources::{
    AdminAuthor, AdminPage, AdminSnippet, AuthorEdit, NewAuthor, NewPage, NewSnippet, PageEdit,
    SnippetEdit,
};
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

#[test]
fn a_snippet_is_sent_in_api_shape_and_edited_field_by_field() {
    let key = (
        Locale::try_new("en").unwrap(),
        Slug::try_new("affiliate-disclosure").unwrap(),
    );
    let snippet = AdminSnippet::new(key, "Affiliate disclosure", "Some links pay us.", at(20));
    let sent = serde_json::to_value(&snippet).unwrap();
    assert_eq!(sent["id"], "en.affiliate-disclosure");
    assert_eq!(sent["title"], "Affiliate disclosure");
    assert_eq!(sent["updated_at"], "2026-10-20T09:00:00Z");
    let edit: SnippetEdit = serde_json::from_value(json!({
        "markdown": "Some links earn us a small fee.", "updated_at": "2026-10-20T09:00:00Z",
    }))
    .unwrap();
    assert_eq!(AdminSnippet::seen(&edit), at(20));
    let edited = serde_json::to_value(snippet.edited(edit, at(27))).unwrap();
    assert_eq!(edited["title"], "Affiliate disclosure");
    assert_eq!(edited["markdown"], "Some links earn us a small fee.");
    assert_eq!(edited["updated_at"], "2026-10-27T09:00:00Z");
}

#[test]
fn a_new_snippet_is_made_in_its_language_and_a_bad_slug_is_refused() {
    let new: NewSnippet = serde_json::from_value(json!({
        "language": "de", "slug": "werbung", "title": "Werbung", "markdown": "Bezahlte Links.",
    }))
    .unwrap();
    let language = AdminSnippet::language(&new).cloned().unwrap();
    let made = AdminSnippet::made(new, language, at(27));
    assert_eq!(made.id(), "de.werbung");
    let refused = serde_json::from_value::<NewSnippet>(json!({
        "slug": "Not a slug", "title": "T", "markdown": "M.",
    }));
    assert!(refused.is_err(), "{refused:?}");
}

#[cfg(test)]
fn asif() -> AdminAuthor {
    let new: NewAuthor = serde_json::from_value(json!({
        "slug": "asif", "name": "Asif", "role": "Founder of SerpDad",
        "same_as": ["https://serpdad.com/"], "bio": "Asif writes about search.",
    }))
    .unwrap();
    AdminAuthor::made(new, Locale::try_new("en").unwrap(), at(20))
}

#[test]
fn an_author_is_sent_in_api_shape() {
    assert_eq!(
        serde_json::to_value(asif()).unwrap(),
        json!({
            "id": "en.asif",
            "slug": "asif",
            "language": "en",
            "name": "Asif",
            "role": "Founder of SerpDad",
            "credentials": null,
            "same_as": ["https://serpdad.com/"],
            "photo": null,
            "bio": "Asif writes about search.",
            "updated_at": "2026-10-20T09:00:00Z",
        })
    );
}

#[test]
fn an_author_edit_changes_only_what_it_sends_and_empty_text_removes() {
    let edit: AuthorEdit = serde_json::from_value(json!({
        "role": "", "credentials": "Ten years in SEO",
        "same_as": ["https://example.com/asif/", "https://serpdad.com/"],
        "updated_at": "2026-10-20T09:00:00Z",
    }))
    .unwrap();
    assert_eq!(AdminAuthor::seen(&edit), at(20));
    let author = serde_json::to_value(asif().edited(edit, at(27))).unwrap();
    assert_eq!(author["name"], "Asif");
    assert_eq!(author["role"], json!(null), "an empty role removes it");
    assert_eq!(author["credentials"], "Ten years in SEO");
    assert_eq!(author["same_as"][0], "https://example.com/asif/");
    assert_eq!(author["bio"], "Asif writes about search.");
    assert_eq!(author["updated_at"], "2026-10-27T09:00:00Z");
}

#[test]
fn a_new_author_needs_a_slug_and_full_addresses() {
    let bare: NewAuthor =
        serde_json::from_value(json!({"slug": "lena-vogt", "name": "Lena Vogt"})).unwrap();
    assert_eq!(bare.file().bio, "", "a bio may come later");
    for bad in [
        json!({"slug": "Lena Vogt", "name": "Lena Vogt"}),
        json!({"slug": "lena-vogt", "name": "Lena Vogt", "same_as": ["not an address"]}),
        json!({"slug": "lena-vogt"}),
    ] {
        let refused = serde_json::from_value::<NewAuthor>(bad.clone());
        assert!(refused.is_err(), "{bad} was read as {refused:?}");
    }
}
