//! A file in the media library, as the Admin API sends it.

use chrono::{DateTime, SecondsFormat, Utc};
use khalid_media::{FileAddress, Kind, MediaItem};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::pagination::Meta;
use crate::query::{FieldValues, Literal};

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
    "url": "https://www.example.com/blog/media/library/3f2a9c1e0b57d0aa/choosing-a-crm.jpg",
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

/// A change to an item's details. A field left out stays as it is.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MediaEditBody {
    /// The `updated_at` of the copy the edit was made on.
    pub updated_at: DateTime<Utc>,
    /// New alt text, up to 1000 characters; empty clears it.
    pub alt: Option<String>,
    /// A new caption, up to 2000 characters; empty clears it.
    pub caption: Option<String>,
    /// A new focal point.
    pub focal_point: Option<FocalPointBody>,
    /// A new folder, such as `guides/2026`, or empty for the top.
    pub folder: Option<String>,
}

/// An edit, wrapped the way the Admin API wraps resources: exactly one.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MediaEdits {
    /// The one edit.
    pub media: Vec<MediaEditBody>,
}

/// Media items, wrapped the way the Admin API wraps resources.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct MediaItems {
    /// The item or items.
    pub media: Vec<AdminMedia>,
}

/// A page of the media library with its pagination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct MediaList {
    /// The items on this page.
    pub media: Vec<AdminMedia>,
    /// Where this page sits in the list.
    pub meta: Meta,
}

impl AdminMedia {
    /// The fields a list's `filter` and `order` may name.
    pub const FIELDS: &'static [&'static str] = &[
        "id",
        "name",
        "folder",
        "type",
        "kind",
        "bytes",
        "width",
        "height",
        "alt",
        "caption",
        "uploaded_at",
        "uploaded_by",
        "updated_at",
        "updated_by",
    ];

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

impl FieldValues for AdminMedia {
    /// Empty alt text or caption has no value, so `alt:null` finds every
    /// image still waiting for its alt text.
    fn values(&self, field: &str) -> Vec<Literal> {
        let text = |value: &str| match value {
            "" => Vec::new(),
            value => vec![Literal::Text(value.to_owned())],
        };
        let number = |value: Option<u64>| {
            value
                .and_then(|value| i64::try_from(value).ok())
                .map(Literal::Integer)
                .into_iter()
                .collect()
        };
        let time = |at: &DateTime<Utc>| text(&at.to_rfc3339_opts(SecondsFormat::Secs, true));
        match field {
            "id" => text(&self.id),
            "name" => text(&self.name),
            "folder" => text(&self.folder),
            "type" => text(&self.media_type),
            "kind" => text(&self.kind),
            "bytes" => number(Some(self.bytes)),
            "width" => number(self.width.map(u64::from)),
            "height" => number(self.height.map(u64::from)),
            "alt" => text(&self.alt),
            "caption" => text(&self.caption),
            "uploaded_at" => time(&self.uploaded_at),
            "uploaded_by" => text(&self.uploaded_by),
            "updated_at" => time(&self.updated_at),
            "updated_by" => text(&self.updated_by),
            _ => Vec::new(),
        }
    }
}
