//! Channels: filtered lists with their own address, such as `/featured/`.

use khalid_core::Slug;
use serde::Deserialize;

/// A filtered list of posts with its own page and feed. A channel never
/// changes a post's own address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Channel {
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) filter: ChannelFilter,
}

impl Channel {
    /// The channel's name, the key under `[channels]`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Its page, relative to the mount path and language prefix, such as
    /// `/featured/`.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Which posts it lists.
    pub fn filter(&self) -> &ChannelFilter {
        &self.filter
    }
}

/// Which posts a channel lists. A post must match every field that is set,
/// and at least one field is set.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct ChannelFilter {
    /// Posts by this author.
    pub author: Option<Slug>,
    /// Posts with this tag.
    pub tag: Option<Slug>,
    /// Posts in this category.
    pub category: Option<Slug>,
    /// Posts in this collection, such as `posts`.
    pub collection: Option<String>,
    /// Posts whose front matter says `featured: true`, or `false` for the rest.
    pub featured: Option<bool>,
}

impl ChannelFilter {
    /// Whether no field is set, which would list every post.
    pub(crate) fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}
