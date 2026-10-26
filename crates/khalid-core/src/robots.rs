//! Robots directives: what a page's `<meta name="robots">` tag tells search
//! engines, read from the same words a writer puts in front matter.

use std::collections::BTreeSet;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer};

/// Whether search engines may list the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
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
    const ALL: [Self; 4] = [
        Self::NoArchive,
        Self::NoSnippet,
        Self::NoImageIndex,
        Self::NoTranslate,
    ];

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

impl PreviewLimit {
    fn parse(value: &str) -> Option<Self> {
        if value == "-1" {
            return Some(Self::Unlimited);
        }
        value.parse().ok().map(Self::AtMost)
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

    fn parse(value: &str) -> Option<Self> {
        [Self::None, Self::Standard, Self::Large]
            .into_iter()
            .find(|size| size.as_str() == value)
    }
}

/// The content of a page's `<meta name="robots">` tag.
///
/// The default lets search engines index the page, follow its links and
/// show snippets and previews of any size. It reads from the words of the
/// tag, such as `noindex, nofollow`, with [`str::parse`], and from a string
/// in front matter or settings.
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

impl FromStr for RobotsDirectives {
    type Err = ParseRobotsError;

    /// Reads directives separated by commas, in any case. Anything not
    /// named keeps its default.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let mut parsed = Parsed::default();
        let mut words = text
            .split(',')
            .map(str::trim)
            .filter(|word| !word.is_empty());
        let Some(first) = words.next() else {
            return Err(ParseRobotsError::Empty);
        };
        parsed.word(first)?;
        for word in words {
            parsed.word(word)?;
        }
        Ok(parsed.into_directives())
    }
}

impl<'de> Deserialize<'de> for RobotsDirectives {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

/// The directives read so far, each with the word that set it, so a
/// contradiction can name both words.
#[derive(Default)]
struct Parsed<'a> {
    indexing: Option<(Indexing, &'a str)>,
    following: Option<(Following, &'a str)>,
    restrictions: BTreeSet<Restriction>,
    max_snippet: Option<PreviewLimit>,
    max_image_preview: Option<ImagePreview>,
    max_video_preview: Option<PreviewLimit>,
}

impl<'a> Parsed<'a> {
    fn word(&mut self, word: &'a str) -> Result<(), ParseRobotsError> {
        let lower = word.to_ascii_lowercase();
        match lower.as_str() {
            "index" => self.indexing(Indexing::Index, word),
            "noindex" => self.indexing(Indexing::NoIndex, word),
            "follow" => self.following(Following::Follow, word),
            "nofollow" => self.following(Following::NoFollow, word),
            "all" => {
                self.indexing(Indexing::Index, word)?;
                self.following(Following::Follow, word)
            }
            "none" => {
                self.indexing(Indexing::NoIndex, word)?;
                self.following(Following::NoFollow, word)
            }
            _ => {
                if let Some(restriction) = Restriction::ALL
                    .into_iter()
                    .find(|restriction| restriction.as_str() == lower)
                {
                    self.restrictions.insert(restriction);
                    return Ok(());
                }
                self.limit(word, &lower)
            }
        }
    }

    fn indexing(&mut self, value: Indexing, word: &'a str) -> Result<(), ParseRobotsError> {
        match self.indexing {
            Some((set, first)) if set != value => Err(contradicts(first, word)),
            _ => {
                self.indexing = Some((value, word));
                Ok(())
            }
        }
    }

    fn following(&mut self, value: Following, word: &'a str) -> Result<(), ParseRobotsError> {
        match self.following {
            Some((set, first)) if set != value => Err(contradicts(first, word)),
            _ => {
                self.following = Some((value, word));
                Ok(())
            }
        }
    }

    /// `max-snippet:160` and the other two directives with a value.
    fn limit(&mut self, word: &str, lower: &str) -> Result<(), ParseRobotsError> {
        let (name, value) = lower.split_once(':').unwrap_or((lower, ""));
        let (name, value) = (name.trim(), value.trim());
        let bad = || ParseRobotsError::BadValue {
            directive: name.to_owned(),
            value: value.to_owned(),
        };
        let repeated = || ParseRobotsError::Repeated {
            name: name.to_owned(),
        };
        match name {
            "max-snippet" | "max-video-preview" => {
                let limit = PreviewLimit::parse(value).ok_or_else(bad)?;
                let slot = if name == "max-snippet" {
                    &mut self.max_snippet
                } else {
                    &mut self.max_video_preview
                };
                if slot.replace(limit).is_some() {
                    return Err(repeated());
                }
                Ok(())
            }
            "max-image-preview" => {
                let size = ImagePreview::parse(value).ok_or_else(bad)?;
                if self.max_image_preview.replace(size).is_some() {
                    return Err(repeated());
                }
                Ok(())
            }
            _ => Err(ParseRobotsError::Unknown {
                word: word.to_owned(),
            }),
        }
    }

    fn into_directives(self) -> RobotsDirectives {
        let default = RobotsDirectives::default();
        RobotsDirectives {
            indexing: self.indexing.map_or(default.indexing, |(value, _)| value),
            following: self.following.map_or(default.following, |(value, _)| value),
            restrictions: self.restrictions,
            max_snippet: self.max_snippet.unwrap_or(default.max_snippet),
            max_image_preview: self.max_image_preview.unwrap_or(default.max_image_preview),
            max_video_preview: self.max_video_preview.unwrap_or(default.max_video_preview),
        }
    }
}

fn contradicts(first: &str, second: &str) -> ParseRobotsError {
    ParseRobotsError::Contradicts {
        first: first.to_owned(),
        second: second.to_owned(),
    }
}

/// Why text could not be read as robots directives.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ParseRobotsError {
    /// No directive was written.
    #[error("the robots list is empty; write directives such as noindex, nofollow")]
    Empty,
    /// A word is not a robots directive.
    #[error(
        "{word:?} is not a robots directive; use index, noindex, follow, nofollow, noarchive, \
         nosnippet, noimageindex, notranslate, max-snippet, max-image-preview or max-video-preview"
    )]
    Unknown {
        /// The word as written.
        word: String,
    },
    /// Two directives say opposite things, such as `index` and `noindex`.
    #[error("{first:?} and {second:?} contradict each other; keep the one you mean")]
    Contradicts {
        /// The first word, as written.
        first: String,
        /// The word that contradicts it, as written.
        second: String,
    },
    /// A directive with a value is given twice.
    #[error("{name} is given twice; keep one value")]
    Repeated {
        /// The directive, such as `max-snippet`.
        name: String,
    },
    /// A directive's value is not allowed.
    #[error(
        "{directive}:{value:?} is not allowed; max-snippet and max-video-preview take -1 or a \
         whole number, and max-image-preview takes none, standard or large"
    )]
    BadValue {
        /// The directive, such as `max-snippet`.
        directive: String,
        /// The value after the colon, empty when there is none.
        value: String,
    },
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
