//! A media item's details live beside it in `media/<key>/item.toml`
//! (ADR "the media library keeps each file with its details").

use chrono::{TimeZone, Utc};
use khalid_media::{ContentKey, FocalPoint, MediaItem, MediaType, Pixels};

const ITEM: &str = r#"name = "choosing-a-crm.jpg"
folder = "guides/2026"
type = "image/jpeg"
bytes = 48213
width = 1600
height = 900
alt = "A sales team comparing two CRM dashboards on one screen"
caption = "Pipedrive on the left, HubSpot on the right."
focal_x = 620
focal_y = 350
uploaded_at = "2026-10-27T08:00:00Z"
uploaded_by = "asif"
updated_at = "2026-10-27T09:30:00Z"
updated_by = "editor"
"#;

#[cfg(test)]
fn id() -> ContentKey {
    ContentKey::parse("3f2a9c1e0b57d0aa").unwrap()
}

#[test]
fn reads_an_item_file_and_writes_it_back_the_same() {
    let item = MediaItem::from_toml(id(), ITEM).unwrap();
    assert_eq!(item.id, id());
    assert_eq!(item.name.as_str(), "choosing-a-crm.jpg");
    assert_eq!(item.folder.as_str(), "guides/2026");
    assert_eq!(item.media_type, MediaType::Jpeg);
    let pixels = Pixels {
        width: 1600,
        height: 900,
    };
    assert_eq!(item.pixels, Some(pixels));
    assert_eq!(item.focal_point, FocalPoint::try_new(620, 350).unwrap());
    let uploaded = Utc.with_ymd_and_hms(2026, 10, 27, 8, 0, 0).unwrap();
    assert_eq!(item.uploaded_at, uploaded);
    assert_eq!(item.to_toml(), ITEM);
}

#[test]
fn a_file_that_is_not_an_image_has_no_pixel_size() {
    let text = ITEM
        .replace("width = 1600\nheight = 900\n", "")
        .replace("image/jpeg", "application/pdf")
        .replace(".jpg", ".pdf");
    let item = MediaItem::from_toml(id(), &text).unwrap();
    assert_eq!(item.pixels, None);
    assert_eq!(item.to_toml(), text);
}

#[test]
fn an_item_file_with_a_wrong_field_is_refused() {
    let cases = [
        ("guides/2026", "../secrets", "the item file is not valid"),
        ("image/jpeg", "text/html", "the item file is not valid"),
        (
            "choosing-a-crm.jpg",
            "choosing a crm.exe",
            "the item file is not valid",
        ),
        ("caption", "colour", "the item file is not valid"),
        ("focal_x = 620", "focal_x = 1001", "focal_x and focal_y"),
        ("height = 900\n", "", "one of width and height"),
    ];
    for (old, new, want) in cases {
        let error = MediaItem::from_toml(id(), &ITEM.replace(old, new)).unwrap_err();
        assert!(error.to_string().contains(want), "{new}: {error}");
    }
}
