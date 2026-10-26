//! German word lists for the rules that read words.
//!
//! `typos.toml` skips this file, because its English dictionary reads German
//! words such as "oder" as typos. Keep only word lists here.

/// Small words that carry no topic.
pub(crate) const STOPWORDS: &[&str] = &[
    "am", "auf", "aus", "das", "dem", "den", "der", "die", "ein", "eine", "einen", "für", "im",
    "in", "ist", "mit", "oder", "und", "von", "zu", "zum", "zur",
];

/// Link words that say nothing about the page they open.
pub(crate) const VAGUE_LINK_TEXT: &[&str] = &[
    "hier",
    "hier klicken",
    "klicken sie hier",
    "mehr",
    "mehr lesen",
    "weiterlesen",
    "dieser artikel",
    "link",
];
