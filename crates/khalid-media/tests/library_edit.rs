//! Alt text, caption, focal point and folder change on the item and are
//! written to its `item.toml`. An edit made on an old copy is refused
//! (RULES.md BL-API-7).

use std::path::Path;
use std::sync::Arc;

use chrono::{TimeZone, Utc};
use khalid_core::{FixedClock, Locale};
use khalid_media::{
    EditMediaError, FileAddress, FocalPoint, Folder, Library, MediaEdit, MediaItem, Storage,
    Upload, Uploaded,
};

const PDF: &[u8] = b"%PDF-1.7\nA checklist for choosing a CRM\n%%EOF\n";

#[cfg(test)]
async fn open(site: &Path, hour: u32) -> Library {
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 10, 27, hour, 0, 0).unwrap());
    let english = Locale::try_new("en").unwrap();
    Library::open(site, Storage::local(site), &english, Arc::new(clock))
        .await
        .unwrap()
}

#[cfg(test)]
async fn uploaded(library: &Library) -> MediaItem {
    let upload = Upload::new("checklist.pdf", Folder::root(), PDF.to_vec(), "asif");
    match library.upload(upload).await.unwrap() {
        Uploaded::New(item) | Uploaded::Duplicate(item) => item,
    }
}

#[tokio::test]
async fn an_edit_changes_only_what_it_names_and_survives_a_restart() {
    let site = tempfile::tempdir().unwrap();
    let item = uploaded(&open(site.path(), 8).await).await;
    let library = open(site.path(), 10).await;
    let mut edit = MediaEdit::new(item.updated_at);
    edit.alt = Some("A one-page checklist with ten questions to ask a CRM vendor".to_owned());
    edit.focal_point = Some(FocalPoint::try_new(250, 100).unwrap());
    edit.folder = Some(Folder::try_new("guides/crm").unwrap());
    let edited = library.edit(&item.id, edit, "editor").await.unwrap();
    assert_eq!(
        edited.alt,
        "A one-page checklist with ten questions to ask a CRM vendor"
    );
    assert_eq!(edited.caption, "", "the caption was not named, so it stays");
    assert_eq!(edited.folder.as_str(), "guides/crm");
    assert_eq!(edited.updated_by, "editor");
    assert_eq!(
        edited.updated_at,
        Utc.with_ymd_and_hms(2026, 10, 27, 10, 0, 0).unwrap()
    );
    assert_eq!(edited.uploaded_at, item.uploaded_at);
    assert_eq!(
        open(site.path(), 11).await.get(&item.id),
        Some(edited.clone())
    );
    assert_eq!(
        library.address(&edited),
        FileAddress::OnSite(format!("media/{}/checklist.pdf", item.id)),
        "moving to another folder keeps the address"
    );
}

#[tokio::test]
async fn an_edit_made_on_an_old_copy_is_refused() {
    let site = tempfile::tempdir().unwrap();
    let item = uploaded(&open(site.path(), 8).await).await;
    let library = open(site.path(), 9).await;
    let mut first = MediaEdit::new(item.updated_at);
    first.caption = Some("Ten questions for any CRM vendor.".to_owned());
    let saved = library.edit(&item.id, first, "editor").await.unwrap();
    let mut late = MediaEdit::new(item.updated_at);
    late.caption = Some("An older caption.".to_owned());
    let error = library.edit(&item.id, late, "asif").await.unwrap_err();
    assert!(
        matches!(&error, EditMediaError::Stale { updated_at, updated_by }
            if *updated_at == saved.updated_at && updated_by == "editor"),
        "{error:?}"
    );
    assert_eq!(library.get(&item.id), Some(saved));
}

#[tokio::test]
async fn refuses_an_unknown_item_and_text_that_is_too_long() {
    let site = tempfile::tempdir().unwrap();
    let library = open(site.path(), 8).await;
    let item = uploaded(&library).await;
    let unknown = khalid_media::ContentKey::of(b"never uploaded");
    let edit = MediaEdit::new(item.updated_at);
    let error = library.edit(&unknown, edit, "asif").await.unwrap_err();
    assert!(matches!(error, EditMediaError::NotFound), "{error:?}");
    let mut long = MediaEdit::new(item.updated_at);
    long.alt = Some("word ".repeat(201));
    let error = library.edit(&item.id, long, "asif").await.unwrap_err();
    assert_eq!(error.to_string(), "alt is longer than 1000 characters");
    assert_eq!(library.get(&item.id), Some(item));
}
