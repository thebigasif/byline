//! The library takes an upload, checks it, stores it once by its hash and
//! keeps its details beside it (plan section 5.15, RULES.md BL-SEC-6).

use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;

use chrono::{TimeZone, Utc};
use image::{ImageFormat, Rgb, RgbImage};
use khalid_core::{FixedClock, Locale};
use khalid_media::{
    ContentKey, Folder, Kind, Library, MediaItem, MediaType, Pixels, Refusal, Storage, Upload,
    UploadMediaError, Uploaded,
};

const PDF: &[u8] = b"%PDF-1.7\nA checklist for choosing a CRM\n%%EOF\n";

#[cfg(test)]
fn png(width: u32, height: u32) -> Vec<u8> {
    let image = RgbImage::from_pixel(width, height, Rgb([30, 90, 160]));
    let mut bytes = Cursor::new(Vec::new());
    image.write_to(&mut bytes, ImageFormat::Png).unwrap();
    bytes.into_inner()
}

/// The library of `site`, with its clock at `hour` o'clock.
#[cfg(test)]
async fn open(site: &Path, hour: u32) -> Library {
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 10, 27, hour, 0, 0).unwrap());
    let english = Locale::try_new("en").unwrap();
    Library::open(site, Storage::local(site), &english, Arc::new(clock))
        .await
        .unwrap()
}

#[cfg(test)]
async fn send(library: &Library, name: &str, bytes: &[u8]) -> Result<Uploaded, UploadMediaError> {
    let upload = Upload::new(
        name,
        Folder::try_new("guides").unwrap(),
        bytes.to_vec(),
        "asif",
    );
    library.upload(upload).await
}

/// Uploads `bytes` as `name` and returns the new item.
#[cfg(test)]
async fn add(library: &Library, name: &str, bytes: &[u8]) -> MediaItem {
    match send(library, name, bytes).await.unwrap() {
        Uploaded::New(item) => item,
        Uploaded::Duplicate(item) => panic!("{} was already there", item.id),
    }
}

#[tokio::test]
async fn an_image_is_stored_with_its_details_and_found_again_after_a_restart() {
    let site = tempfile::tempdir().unwrap();
    let bytes = png(320, 180);
    let item = add(&open(site.path(), 8).await, "Choosing a CRM.PNG", &bytes).await;
    assert_eq!(item.id, ContentKey::of(&bytes));
    assert_eq!(item.name.as_str(), "choosing-a-crm.png");
    assert_eq!(item.folder.as_str(), "guides");
    assert_eq!(item.media_type, MediaType::Png);
    let pixels = Pixels {
        width: 320,
        height: 180,
    };
    assert_eq!(item.pixels, Some(pixels));
    assert_eq!(item.uploaded_by, "asif");
    let eight = Utc.with_ymd_and_hms(2026, 10, 27, 8, 0, 0).unwrap();
    assert_eq!(item.uploaded_at, eight);
    let stored = Storage::local(site.path()).get(&item.id, &item.name).await;
    let stored = stored.unwrap().unwrap();
    assert_eq!(stored.len(), usize::try_from(item.bytes).unwrap());
    assert_eq!(image::guess_format(&stored).unwrap(), ImageFormat::Png);
    assert_eq!(open(site.path(), 9).await.get(&item.id), Some(item));
}

#[tokio::test]
async fn the_same_bytes_again_are_a_duplicate_of_the_first_upload() {
    let site = tempfile::tempdir().unwrap();
    let library = open(site.path(), 8).await;
    let first = add(&library, "checklist.pdf", PDF).await;
    let again = send(&library, "a copy.pdf", PDF).await.unwrap();
    assert_eq!(again, Uploaded::Duplicate(first.clone()));
    assert_eq!(library.items(), [first]);
}

#[tokio::test]
async fn a_file_that_is_not_an_image_is_stored_as_sent() {
    let site = tempfile::tempdir().unwrap();
    let item = add(&open(site.path(), 8).await, "checklist", PDF).await;
    assert_eq!(item.name.as_str(), "checklist.pdf");
    assert_eq!(item.pixels, None);
    let stored = Storage::local(site.path()).get(&item.id, &item.name).await;
    assert_eq!(stored.unwrap().as_deref(), Some(PDF));
}

#[tokio::test]
async fn refuses_what_the_library_must_not_keep() {
    let site = tempfile::tempdir().unwrap();
    let library = open(site.path(), 8).await;
    let mut broken = png(4, 4);
    broken.truncate(40);
    let too_large = vec![b'a'; 20 * 1024 * 1024 + 1];
    let cases: [(&[u8], Refusal); 4] = [
        (b"<!doctype html><script>steal()</script>", Refusal::Markup),
        (b"\x7FELF\x02\x01\x01\x00", Refusal::Program),
        (&broken, Refusal::Broken),
        (&too_large, Refusal::TooLarge { kind: Kind::File }),
    ];
    for (bytes, want) in cases {
        let error = send(&library, "upload", bytes).await.unwrap_err();
        let refused = matches!(error, UploadMediaError::Refused(why) if why == want);
        assert!(refused, "{error:?}");
    }
    assert_eq!(library.items(), []);
}

#[tokio::test]
async fn lists_the_newest_upload_first() {
    let site = tempfile::tempdir().unwrap();
    let first = add(&open(site.path(), 8).await, "first.pdf", PDF).await;
    let noon = open(site.path(), 12).await;
    let second = add(&noon, "second.png", &png(8, 8)).await;
    let ids: Vec<ContentKey> = noon.items().into_iter().map(|item| item.id).collect();
    assert_eq!(ids, [second.id, first.id]);
}
