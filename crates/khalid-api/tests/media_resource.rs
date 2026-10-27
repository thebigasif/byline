//! A media item as the Admin API sends it: its details, its kind, and the
//! full address readers fetch it from, on the site or in a bucket.

use khalid_api::resources::AdminMedia;
use khalid_media::{ContentKey, FileAddress, MediaItem};
use serde_json::{Value, json};

const ITEM: &str = r#"name = "choosing-a-crm.jpg"
folder = "guides"
type = "image/jpeg"
bytes = 48213
width = 1600
height = 900
alt = "A sales team comparing two CRM dashboards on one screen"
caption = ""
focal_x = 620
focal_y = 350
uploaded_at = "2026-10-27T08:00:00Z"
uploaded_by = "asif"
updated_at = "2026-10-27T09:30:00Z"
updated_by = "editor"
"#;

#[cfg(test)]
fn item() -> MediaItem {
    MediaItem::from_toml(ContentKey::parse("3f2a9c1e0b57d0aa").unwrap(), ITEM).unwrap()
}

#[test]
fn an_item_on_the_site_has_the_blogs_address_before_its_path() {
    let path = "media/3f2a9c1e0b57d0aa/choosing-a-crm.jpg".to_owned();
    let shown = AdminMedia::new(
        &item(),
        FileAddress::OnSite(path),
        "https://www.example.com/blog/",
    );
    let json: Value = serde_json::to_value(shown).unwrap();
    assert_eq!(
        json,
        json!({
            "id": "3f2a9c1e0b57d0aa",
            "name": "choosing-a-crm.jpg",
            "folder": "guides",
            "type": "image/jpeg",
            "kind": "image",
            "bytes": 48213,
            "width": 1600,
            "height": 900,
            "alt": "A sales team comparing two CRM dashboards on one screen",
            "caption": "",
            "focal_point": { "x": 620, "y": 350 },
            "url": "https://www.example.com/blog/media/3f2a9c1e0b57d0aa/choosing-a-crm.jpg",
            "uploaded_at": "2026-10-27T08:00:00Z",
            "uploaded_by": "asif",
            "updated_at": "2026-10-27T09:30:00Z",
            "updated_by": "editor"
        })
    );
}

#[test]
fn an_item_in_a_bucket_keeps_the_buckets_address() {
    let url = "https://media.example.com/blog/3f2a9c1e0b57d0aa/choosing-a-crm.jpg";
    let address = FileAddress::Elsewhere(url.to_owned());
    let shown = AdminMedia::new(&item(), address, "https://www.example.com/blog/");
    assert_eq!(shown.url, url);
}
