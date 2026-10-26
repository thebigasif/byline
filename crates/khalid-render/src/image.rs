//! A responsive image: every size and format, with the space it takes reserved.

use serde::{Serialize, Serializer};

/// An image as a page shows it: a `picture` with AVIF and WebP sources and a
/// fallback `img` that always carries `srcset`, `sizes`, `width`, `height`
/// and `alt` (RULES.md BL-WEB-5).
///
/// The media crate produces the files; the wiring code fills this record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RenderImage {
    /// What the image shows, for people who cannot see it. Empty only for
    /// an image that is purely decoration.
    pub alt: String,
    /// The caption shown under the image, if any.
    pub caption: Option<String>,
    /// The original's width in pixels, which reserves the space.
    pub width: u32,
    /// The original's height in pixels, which reserves the space.
    pub height: u32,
    /// How wide the image is drawn, as the `sizes` attribute says it, such as
    /// `(min-width: 48rem) 48rem, 100vw`.
    pub sizes: String,
    /// The AVIF files, if any were made.
    pub avif: Option<ImageSet>,
    /// The WebP files, if any were made.
    pub webp: Option<ImageSet>,
    /// The files in the original format, for browsers that read neither.
    pub fallback: ImageSet,
    /// Whether the browser fetches it at once or when it nears the screen.
    pub loading: Loading,
}

/// When the browser fetches an image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Loading {
    /// At once, with high priority: the one large image at the top of a page.
    Eager,
    /// When it nears the screen: every other image.
    Lazy,
}

/// One format of an image in several widths. Never empty, and always sorted
/// from narrowest to widest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageSet {
    candidates: Vec<ImageCandidate>,
}

/// One file of an image set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageCandidate {
    /// The file's address, such as `/blog/media/3f2a9c1e-800.webp`.
    pub url: String,
    /// Its width in pixels.
    pub width: u32,
}

impl ImageSet {
    /// A set with `first` and every file in `rest`, sorted by width, with
    /// one file kept per width: the earliest given.
    pub fn new(first: ImageCandidate, rest: Vec<ImageCandidate>) -> Self {
        let mut candidates = vec![first];
        candidates.extend(rest);
        candidates.sort_by_key(|candidate| candidate.width);
        candidates.dedup_by_key(|candidate| candidate.width);
        Self { candidates }
    }

    /// The widest file, which `src` points at.
    pub fn widest(&self) -> Option<&ImageCandidate> {
        self.candidates.last()
    }

    /// The set as a `srcset` value: `a.webp 480w, b.webp 960w`.
    pub fn to_srcset(&self) -> String {
        self.candidates
            .iter()
            .map(|candidate| format!("{} {}w", candidate.url, candidate.width))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl Serialize for ImageSet {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Shown<'a> {
            src: &'a str,
            srcset: String,
        }
        Shown {
            src: self.widest().map_or("", |candidate| candidate.url.as_str()),
            srcset: self.to_srcset(),
        }
        .serialize(serializer)
    }
}
