//! Turning or cropping a library picture makes a new copy and leaves the
//! original as it was (plan 4.6, decisions E9 and E10).

use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;

use chrono::{TimeZone, Utc};
use image::{ImageFormat, Rgb, RgbImage};
use khalid_core::{FixedClock, Locale};
use khalid_media::{
    ContentKey, CropBox, EditPictureError, Folder, Library, MediaEdit, MediaItem, PictureEdit,
    Pixels, Storage, Turn, Upload, Uploaded,
};

const RED: Rgb<u8> = Rgb([200, 20, 20]);

/// A blue PNG of `width` by `height` with its top left pixel red.
#[cfg(test)]
fn png(width: u32, height: u32) -> Vec<u8> {
    let mut image = RgbImage::from_pixel(width, height, Rgb([20, 20, 200]));
    image.put_pixel(0, 0, RED);
    let mut bytes = Cursor::new(Vec::new());
    image.write_to(&mut bytes, ImageFormat::Png).unwrap();
    bytes.into_inner()
}

#[cfg(test)]
async fn open(site: &Path) -> Library {
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 10, 31, 9, 0, 0).unwrap());
    let english = Locale::try_new("en").unwrap();
    Library::open(site, Storage::local(site), &english, Arc::new(clock))
        .await
        .unwrap()
}

#[cfg(test)]
async fn add(library: &Library, name: &str, bytes: Vec<u8>) -> MediaItem {
    let upload = Upload::new(name, Folder::try_new("guides").unwrap(), bytes, "asif");
    match library.upload(upload).await.unwrap() {
        Uploaded::New(item) => item,
        Uploaded::Duplicate(item) => panic!("{} was already there", item.id),
    }
}

#[tokio::test]
async fn a_turned_copy_keeps_the_details_and_leaves_the_original() {
    let site = tempfile::tempdir().unwrap();
    let library = open(site.path()).await;
    let item = add(&library, "Team photo.png", png(4, 2)).await;
    let mut details = MediaEdit::new(item.updated_at);
    details.alt = Some("The team at the summer meetup".to_owned());
    let item = library.edit(&item.id, details, "asif").await.unwrap();
    let edit = PictureEdit::new(Turn::Quarter, None);
    let made = library.edit_picture(&item.id, edit, "maria").await.unwrap();
    let Uploaded::New(copy) = made else {
        panic!("a new copy, found {made:?}");
    };
    assert_ne!(copy.id, item.id);
    assert_eq!(
        copy.pixels,
        Some(Pixels {
            width: 2,
            height: 4
        })
    );
    assert_eq!(copy.name, item.name);
    assert_eq!(copy.folder, item.folder);
    assert_eq!(copy.alt, "The team at the summer meetup");
    assert_eq!(copy.uploaded_by, "maria");
    let (_, bytes) = library
        .read(&copy.id, copy.name.as_str())
        .await
        .unwrap()
        .unwrap();
    let turned = image::load_from_memory(&bytes).unwrap().to_rgb8();
    assert_eq!(turned.get_pixel(1, 0), &RED, "top left went to top right");
    assert_eq!(
        library.get(&item.id),
        Some(item.clone()),
        "the original stays"
    );
    let again = library.edit_picture(&item.id, edit, "maria").await.unwrap();
    assert_eq!(again, Uploaded::Duplicate(copy));
}

#[tokio::test]
async fn a_cropped_copy_is_found_again_after_a_restart() {
    let site = tempfile::tempdir().unwrap();
    let library = open(site.path()).await;
    let item = add(&library, "wide.png", png(400, 200)).await;
    let square = CropBox::try_new(250, 0, 500, 1000).unwrap();
    let edit = PictureEdit::new(Turn::None, Some(square));
    let made = library.edit_picture(&item.id, edit, "asif").await.unwrap();
    let Uploaded::New(copy) = made else {
        panic!("a new copy, found {made:?}");
    };
    let reopened = open(site.path()).await;
    let found = reopened.get(&copy.id).unwrap();
    let pixels = found.pixels.unwrap();
    assert_eq!((pixels.width, pixels.height), (200, 200));
    assert_eq!(reopened.items().len(), 2);
}

#[tokio::test]
async fn only_a_picture_the_library_has_can_be_changed() {
    let site = tempfile::tempdir().unwrap();
    let library = open(site.path()).await;
    let pdf = b"%PDF-1.7\nA checklist\n%%EOF\n".to_vec();
    let file = add(&library, "checklist.pdf", pdf).await;
    let edit = PictureEdit::new(Turn::Half, None);
    let answer = library.edit_picture(&file.id, edit, "asif").await;
    assert!(
        matches!(answer, Err(EditPictureError::NotAPicture)),
        "{answer:?}"
    );
    let unknown = ContentKey::of(b"nothing");
    let answer = library.edit_picture(&unknown, edit, "asif").await;
    assert!(
        matches!(answer, Err(EditPictureError::NotFound)),
        "{answer:?}"
    );
}
