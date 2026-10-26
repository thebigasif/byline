//! The numbers behind the rules that measure, set in `lint.toml`.

use serde::Deserialize;

/// The limits a site can tune in the `[thresholds]` table of `lint.toml`.
///
/// Each key is written in kebab-case, such as `thin-post-words = 500`. A key
/// the file leaves out keeps the default documented on its field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
#[non_exhaustive]
pub struct Thresholds {
    /// A post with fewer words than this is thin. Default 300, about the
    /// length of a short answer with one example.
    pub thin_post_words: usize,
    /// How many links site-wide may use the same words for the same page
    /// before the next one is flagged. Default 3, which leaves room for a
    /// short series of posts.
    pub same_link_text_max: usize,
    /// The longest title, in characters, before search results cut it.
    /// Default 60, about what Google shows on a desktop result.
    pub title_max_chars: usize,
    /// The longest description, in characters, before search results cut it.
    /// Default 160, about what Google shows on a desktop result.
    pub description_max_chars: usize,
    /// The share of a post's words, in percent, one repeated phrase may
    /// fill. Default 3, well above what natural writing reaches.
    pub keyword_density_percent: usize,
    /// The longest web address ending, in characters. Default 60, about ten
    /// short words.
    pub slug_max_chars: usize,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            thin_post_words: 300,
            same_link_text_max: 3,
            title_max_chars: 60,
            description_max_chars: 160,
            keyword_density_percent: 3,
            slug_max_chars: 60,
        }
    }
}
