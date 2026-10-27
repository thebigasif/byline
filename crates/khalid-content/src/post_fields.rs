//! The post fields that carry more than one value or a rule of their own:
//! the lead image, the search title, tags and the sitemap choice.

use serde::{Deserialize, Deserializer};

/// The image at the top of a post, also used on its cards and share cards.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "RawImage")]
#[non_exhaustive]
pub struct LeadImage {
    /// Where the image is: a file beside the post such as `cover.jpg`, a
    /// media path such as `/media/cover.jpg`, or a full address.
    pub src: String,
    /// What the image shows, for readers who cannot see it.
    pub alt: String,
    /// The line shown under the image, if any.
    pub caption: Option<String>,
}

impl LeadImage {
    /// The image at `src`, described by `alt`, with no caption. `None` when
    /// either is blank, as front matter refuses.
    #[must_use]
    pub fn try_new(src: &str, alt: &str) -> Option<Self> {
        let image = Self {
            src: src.to_owned(),
            alt: alt.to_owned(),
            caption: None,
        };
        (!src.trim().is_empty() && !alt.trim().is_empty()).then_some(image)
    }
}

/// The lead image as written, before its rules are checked.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawImage {
    src: String,
    alt: String,
    #[serde(default)]
    caption: Option<String>,
}

impl TryFrom<RawImage> for LeadImage {
    type Error = &'static str;

    fn try_from(raw: RawImage) -> Result<Self, Self::Error> {
        if raw.src.trim().is_empty() {
            return Err("the image needs a src, such as src: cover.jpg for a file beside the post");
        }
        if raw.alt.trim().is_empty() {
            return Err(
                "the image needs alt text that says what it shows, such as alt: A map of internal links",
            );
        }
        let caption = raw.caption.filter(|caption| !caption.trim().is_empty());
        Ok(Self {
            src: raw.src,
            alt: raw.alt,
            caption,
        })
    }
}

/// Whether a post is listed in the XML sitemap.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(from = "bool")]
#[non_exhaustive]
pub enum Sitemap {
    /// Listed, as every post is unless it says otherwise.
    #[default]
    Listed,
    /// Left out, because the front matter says `sitemap: false`.
    Unlisted,
}

impl From<bool> for Sitemap {
    fn from(listed: bool) -> Self {
        if listed { Self::Listed } else { Self::Unlisted }
    }
}

/// Reads the search title: one phrase with at least one word.
pub(crate) fn seo_title<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<String>, D::Error> {
    let text = String::deserialize(deserializer)?;
    if text.trim().is_empty() {
        return Err(serde::de::Error::custom(
            "the search title needs at least one word; remove the field to use the post's title",
        ));
    }
    Ok(Some(text))
}

/// Reads the tags: one name or a list, each with at least one word and
/// none named twice, whatever its capitals.
pub(crate) fn tags<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<String>, D::Error> {
    let tags = crate::post::one_or_many(deserializer)?;
    let mut seen: Vec<String> = Vec::with_capacity(tags.len());
    for tag in &tags {
        if tag.trim().is_empty() {
            return Err(serde::de::Error::custom(
                "a tag needs at least one word, such as internal links; remove the empty one",
            ));
        }
        let key = tag.trim().to_lowercase();
        if seen.contains(&key) {
            return Err(serde::de::Error::custom(format!(
                "the tag {tag:?} is listed twice; keep one"
            )));
        }
        seen.push(key);
    }
    Ok(tags)
}
