//! A file in the media library, as the Admin API sends it.

use chrono::{DateTime, Utc};
use khalid_media::{FileAddress, Kind, MediaItem};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A point in an image in thousandths of its width and height, from the top
/// left. `{ "x": 500, "y": 500 }` is the centre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct FocalPointBody {
    /// Thousandths of the width from the left, 0 to 1000.
    pub x: u16,
    /// Thousandths of the height from the top, 0 to 1000.
    pub y: u16,
}

/// A file in the media library with the details kept beside it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
#[schema(example = json!({
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
    "focal_point": { "x": 500, "y": 500 },
    "url": "https://www.example.com/blog/media/3f2a9c1e0b57d0aa/choosing-a-crm.jpg",
    "uploaded_at": "2026-10-27T08:00:00Z",
    "uploaded_by": "asif",
    "updated_at": "2026-10-27T08:00:00Z",
    "updated_by": "asif"
}))]
pub struct AdminMedia {
    /// A hash of the uploaded bytes, the same for the same file.
    pub id: String,
    /// The file name it is served under.
    pub name: String,
    /// The library folder, or empty for the top.
    pub folder: String,
    /// The `Content-Type`, read from the bytes.
    #[serde(rename = "type")]
    pub media_type: String,
    /// `image`, `video`, `audio` or `file`.
    pub kind: String,
    /// The size in bytes as stored.
    pub bytes: u64,
    /// The width in pixels, for images.
    pub width: Option<u32>,
    /// The height in pixels, for images.
    pub height: Option<u32>,
    /// What an image shows, for readers who cannot see it.
    pub alt: String,
    /// The line shown under it.
    pub caption: String,
    /// The part of an image every crop keeps.
    pub focal_point: FocalPointBody,
    /// Where readers fetch the file.
    pub url: String,
    /// When it was uploaded.
    pub uploaded_at: DateTime<Utc>,
    /// Who uploaded it.
    pub uploaded_by: String,
    /// When its details last changed. An edit sends this back.
    pub updated_at: DateTime<Utc>,
    /// Who changed them.
    pub updated_by: String,
}

/// Media items, wrapped the way the Admin API wraps resources.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct MediaItems {
    /// The item or items.
    pub media: Vec<AdminMedia>,
}

impl AdminMedia {
    /// `item` as the API sends it, fetched at `address`, with a file on the
    /// site itself under `site`, such as `https://www.example.com/blog/`.
    pub fn new(item: &MediaItem, address: FileAddress, site: &str) -> Self {
        let url = match address {
            FileAddress::OnSite(path) => format!("{site}{path}"),
            FileAddress::Elsewhere(url) => url,
            _ => String::new(),
        };
        let kind = match item.media_type.kind() {
            Kind::Image => "image",
            Kind::Video => "video",
            Kind::Audio => "audio",
            Kind::File => "file",
        };
        Self {
            id: item.id.to_string(),
            name: item.name.as_str().to_owned(),
            folder: item.folder.as_str().to_owned(),
            media_type: item.media_type.mime().to_owned(),
            kind: kind.to_owned(),
            bytes: item.bytes,
            width: item.pixels.map(|pixels| pixels.width),
            height: item.pixels.map(|pixels| pixels.height),
            alt: item.alt.clone(),
            caption: item.caption.clone(),
            focal_point: FocalPointBody {
                x: item.focal_point.x(),
                y: item.focal_point.y(),
            },
            url,
            uploaded_at: item.uploaded_at,
            uploaded_by: item.uploaded_by.clone(),
            updated_at: item.updated_at,
            updated_by: item.updated_by.clone(),
        }
    }
}
