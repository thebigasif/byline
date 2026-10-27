//! Local storage keeps a file's bytes in the site's `media/<key>/` folder,
//! beside its `item.toml` (ADR "the media library keeps each file with its
//! details").

use std::fs;

use khalid_core::Locale;
use khalid_media::{ContentKey, FileAddress, FileName, MediaType, Storage};

const PDF: &[u8] = b"%PDF-1.7\nA checklist for choosing a CRM\n%%EOF\n";

#[cfg(test)]
fn name() -> FileName {
    let english = Locale::try_new("en").unwrap();
    FileName::new("CRM checklist.pdf", MediaType::Pdf, &english)
}

#[tokio::test]
async fn keeps_a_file_whole_in_its_folder_and_reads_it_back() {
    let site = tempfile::tempdir().unwrap();
    let storage = Storage::local(site.path());
    let key = ContentKey::of(PDF);
    storage.put(&key, &name(), PDF.to_vec()).await.unwrap();
    let mut folder = site.path().to_path_buf();
    folder.push("media");
    folder.push(key.as_str());
    let files: Vec<String> = fs::read_dir(&folder)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(files, ["crm-checklist.pdf"], "no partial file is left");
    assert_eq!(
        storage.get(&key, &name()).await.unwrap(),
        Some(PDF.to_vec())
    );
}

#[tokio::test]
async fn a_file_never_stored_reads_as_none() {
    let site = tempfile::tempdir().unwrap();
    let storage = Storage::local(site.path());
    let key = ContentKey::of(b"never uploaded");
    assert_eq!(storage.get(&key, &name()).await.unwrap(), None);
}

#[tokio::test]
async fn storing_the_same_file_twice_keeps_one_copy() {
    let site = tempfile::tempdir().unwrap();
    let storage = Storage::local(site.path());
    let key = ContentKey::of(PDF);
    storage.put(&key, &name(), PDF.to_vec()).await.unwrap();
    storage.put(&key, &name(), PDF.to_vec()).await.unwrap();
    assert_eq!(
        storage.get(&key, &name()).await.unwrap(),
        Some(PDF.to_vec())
    );
}

#[test]
fn a_local_file_is_served_by_the_site_under_media() {
    let storage = Storage::local(std::path::Path::new("/srv/field-notes"));
    let key = ContentKey::parse("3f2a9c1e0b57d0aa").unwrap();
    assert_eq!(
        storage.address(&key, &name()),
        FileAddress::OnSite("media/3f2a9c1e0b57d0aa/crm-checklist.pdf".to_owned())
    );
}
