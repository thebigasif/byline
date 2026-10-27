//! One item of a `WordPress` export, with its text as written.

use chrono::NaiveDateTime;
use khalid_core::AbsoluteUrl;

/// One item of the export.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Item {
    /// The id, which other items use to point at this one.
    pub id: u64,
    /// What kind of item it is.
    pub kind: ItemKind,
    /// Whether it was published.
    pub status: ItemStatus,
    /// The headline.
    pub title: String,
    /// The slug, which may be percent-encoded, or empty for an unsaved draft.
    pub name: String,
    /// The address readers knew it by.
    pub link: Option<AbsoluteUrl>,
    /// The author's sign-in name.
    pub creator: String,
    /// When it was published, in the site's own time zone. None for no date.
    pub published: Option<NaiveDateTime>,
    /// The body, as HTML.
    pub content: String,
    /// The hand-written summary, which may be empty.
    pub excerpt: String,
    /// Whether the post was pinned to the top, which Khalid calls featured.
    pub sticky: bool,
    /// The item this one belongs to, such as the post an image was added to.
    pub parent: Option<u64>,
    /// The file's address, for an attachment.
    pub attachment_url: Option<AbsoluteUrl>,
    /// The slugs of its categories, in file order.
    pub categories: Vec<String>,
    /// The slugs of its tags, in file order.
    pub tags: Vec<String>,
    /// Every custom field in file order, where SEO plugins keep their data.
    pub meta: Vec<Meta>,
}

impl Item {
    /// The value of the first custom field called `key`.
    pub fn meta(&self, key: &str) -> Option<&str> {
        self.meta
            .iter()
            .find(|meta| meta.key == key)
            .map(|meta| meta.value.as_str())
    }
}

/// One custom field of an item.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Meta {
    /// The field's name, such as `_yoast_wpseo_title`.
    pub key: String,
    /// The field's value, as stored.
    pub value: String,
}

/// What kind of item an export holds.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ItemKind {
    /// A blog post.
    Post,
    /// A page, such as About.
    Page,
    /// An uploaded file, such as an image.
    Attachment,
    /// Anything else, such as a menu item, by its name in the file.
    Other(String),
}

impl ItemKind {
    pub(crate) fn from_name(name: &str) -> Self {
        match name {
            "post" => Self::Post,
            "page" => Self::Page,
            "attachment" => Self::Attachment,
            other => Self::Other(other.to_owned()),
        }
    }
}

/// Whether an item was published.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ItemStatus {
    /// Live on the site.
    Publish,
    /// Scheduled for later.
    Future,
    /// Being written.
    Draft,
    /// Waiting for review.
    Pending,
    /// Visible only to signed-in editors.
    Private,
    /// In the bin.
    Trash,
    /// Takes the status of its parent, as attachments do.
    Inherit,
    /// Any other status, by its name in the file.
    Other(String),
}

impl ItemStatus {
    pub(crate) fn from_name(name: &str) -> Self {
        match name {
            "publish" => Self::Publish,
            "future" => Self::Future,
            "draft" => Self::Draft,
            "pending" => Self::Pending,
            "private" => Self::Private,
            "trash" => Self::Trash,
            "inherit" => Self::Inherit,
            other => Self::Other(other.to_owned()),
        }
    }
}
