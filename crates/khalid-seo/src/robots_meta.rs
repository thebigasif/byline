//! Robots directives per page and the site's index policy per page type.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::{PageKind, SeoPage};

/// Whether search engines may list the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Indexing {
    /// The page may appear in search results.
    #[default]
    Index,
    /// The page must not appear in search results.
    NoIndex,
}

/// Whether search engines may follow the page's links.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Following {
    /// Links on the page may be followed.
    #[default]
    Follow,
    /// Links on the page must not be followed.
    NoFollow,
}

/// A use of the page that search engines must not make.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Restriction {
    /// No cached copy: `noarchive`.
    NoArchive,
    /// No text snippet or video preview: `nosnippet`.
    NoSnippet,
    /// The page's images are not indexed: `noimageindex`.
    NoImageIndex,
    /// No offer to translate the page: `notranslate`.
    NoTranslate,
}

impl Restriction {
    fn as_str(self) -> &'static str {
        match self {
            Self::NoArchive => "noarchive",
            Self::NoSnippet => "nosnippet",
            Self::NoImageIndex => "noimageindex",
            Self::NoTranslate => "notranslate",
        }
    }
}

/// How long a snippet or video preview may be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewLimit {
    /// As long as the search engine likes: `-1`.
    Unlimited,
    /// At most this many characters or seconds. `0` means none.
    AtMost(u32),
}

impl fmt::Display for PreviewLimit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unlimited => f.write_str("-1"),
            Self::AtMost(limit) => write!(f, "{limit}"),
        }
    }
}

/// How large an image preview may be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImagePreview {
    /// No image preview.
    None,
    /// A default-size preview.
    Standard,
    /// A large preview, as Google Discover shows.
    Large,
}

impl ImagePreview {
    fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Standard => "standard",
            Self::Large => "large",
        }
    }
}

/// The content of a page's `<meta name="robots">` tag.
///
/// The default lets search engines index the page, follow its links and
/// show snippets and previews of any size.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RobotsDirectives {
    /// `index` or `noindex`.
    pub indexing: Indexing,
    /// `follow` or `nofollow`.
    pub following: Following,
    /// `noarchive`, `nosnippet` and the rest, in a fixed order.
    pub restrictions: BTreeSet<Restriction>,
    /// `max-snippet`, in characters.
    pub max_snippet: PreviewLimit,
    /// `max-image-preview`.
    pub max_image_preview: ImagePreview,
    /// `max-video-preview`, in seconds.
    pub max_video_preview: PreviewLimit,
}

impl Default for RobotsDirectives {
    fn default() -> Self {
        Self {
            indexing: Indexing::Index,
            following: Following::Follow,
            restrictions: BTreeSet::new(),
            max_snippet: PreviewLimit::Unlimited,
            max_image_preview: ImagePreview::Large,
            max_video_preview: PreviewLimit::Unlimited,
        }
    }
}

impl fmt::Display for RobotsDirectives {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.indexing {
            Indexing::Index => "index",
            Indexing::NoIndex => "noindex",
        })?;
        f.write_str(match self.following {
            Following::Follow => ", follow",
            Following::NoFollow => ", nofollow",
        })?;
        if self.indexing == Indexing::NoIndex {
            return Ok(());
        }
        for restriction in &self.restrictions {
            write!(f, ", {}", restriction.as_str())?;
        }
        write!(
            f,
            ", max-snippet:{}, max-image-preview:{}, max-video-preview:{}",
            self.max_snippet,
            self.max_image_preview.as_str(),
            self.max_video_preview
        )
    }
}

/// Which page types search engines may index, and when an archive is too thin.
///
/// By default search results, date archives and the not-found page are
/// `noindex`, and so is a tag or author archive with fewer than three posts.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct IndexPolicy {
    /// The setting per page type. A type not listed is indexed.
    pub by_kind: BTreeMap<PageKind, Indexing>,
    /// A tag or author archive listing fewer posts than this is `noindex`.
    pub thin_archive_below: u32,
}

impl Default for IndexPolicy {
    fn default() -> Self {
        Self {
            by_kind: BTreeMap::from([
                (PageKind::Search, Indexing::NoIndex),
                (PageKind::Date, Indexing::NoIndex),
                (PageKind::NotFound, Indexing::NoIndex),
            ]),
            thin_archive_below: 3,
        }
    }
}

impl IndexPolicy {
    /// The directives for `page`: its own when it has them, otherwise the
    /// defaults with this policy's index setting.
    pub fn directives_for(&self, page: &SeoPage) -> RobotsDirectives {
        if let Some(own) = &page.robots {
            return own.clone();
        }
        let thin = matches!(page.kind, PageKind::Tag | PageKind::Author)
            && page
                .item_count
                .is_some_and(|count| count < self.thin_archive_below);
        let indexing = if thin {
            Indexing::NoIndex
        } else {
            self.by_kind.get(&page.kind).copied().unwrap_or_default()
        };
        RobotsDirectives {
            indexing,
            ..RobotsDirectives::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ImagePreview, PreviewLimit, Restriction, RobotsDirectives};

    #[test]
    fn restrictions_and_limits_print_in_a_fixed_order() {
        let directives = RobotsDirectives {
            restrictions: [Restriction::NoTranslate, Restriction::NoArchive].into(),
            max_snippet: PreviewLimit::AtMost(160),
            max_image_preview: ImagePreview::Standard,
            max_video_preview: PreviewLimit::AtMost(0),
            ..RobotsDirectives::default()
        };
        assert_eq!(
            directives.to_string(),
            "index, follow, noarchive, notranslate, max-snippet:160, \
             max-image-preview:standard, max-video-preview:0"
        );
    }
}
