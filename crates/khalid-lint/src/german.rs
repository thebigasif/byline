//! German word lists for the rules that read words.
//!
//! `typos.toml` skips this file, because its English dictionary reads German
//! words such as "oder" as typos. Keep only word lists here.

/// Small words that carry no topic.
pub(crate) const STOPWORDS: &[&str] = &[
    "als", "am", "auch", "auf", "aus", "bei", "das", "dass", "dem", "den", "der", "des", "die",
    "ein", "eine", "einen", "einer", "eines", "es", "für", "hat", "ihr", "im", "in", "ist", "kann",
    "man", "mit", "nach", "nicht", "noch", "nur", "oder", "sich", "sie", "sind", "so", "um", "und",
    "von", "was", "wenn", "wie", "wir", "wird", "zu", "zum", "zur",
];

/// Short forms that end with a full stop inside a sentence, in lowercase.
pub(crate) const ABBREVIATIONS: &[&str] = &[
    "z.", "b.", "z.b.", "d.h.", "u.a.", "usw.", "bzw.", "ca.", "dr.", "nr.", "vgl.", "evtl.",
    "ggf.", "inkl.", "s.",
];

/// German sentences whose only repeated phrase is grammar, for the tests.
#[cfg(test)]
pub(crate) const GRAMMAR_ONLY: &str =
    "Nutzen Sie Links. Nutzen Sie Titel. Nutzen Sie Bilder. Fertig.";

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
