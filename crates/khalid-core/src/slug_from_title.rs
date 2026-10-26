//! Making a slug from a post's title.

use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

use crate::slug::encoded_len;
use crate::{Locale, ParseSlugError, Slug, SlugMode};

impl Slug {
    /// Makes a slug from `title`, written in `locale`, the way the site's
    /// `mode` asks (`docs/decisions/2026-10-26-cto-answers.md`, section 2).
    ///
    /// In [`SlugMode::Ascii`], German `ä ö ü ß` become `ae oe ue ss`, other
    /// Latin letters lose their accents, and apostrophes vanish, so
    /// "Don't panic" becomes `dont-panic`. A title with no letters that have
    /// an ASCII form, such as Arabic or Japanese, takes the Unicode path
    /// instead. In [`SlugMode::Unicode`] letters keep their accents in
    /// lowercase. Everything that is not a letter or digit becomes a word
    /// break, and a long title is cut at a word so the slug fits
    /// [`Slug::MAX_LEN`].
    ///
    /// # Examples
    ///
    /// ```
    /// use khalid_core::{Locale, Slug, SlugMode};
    ///
    /// let german = Locale::try_new("de")?;
    /// let slug = Slug::from_title("Über Größe", &german, SlugMode::Ascii)?;
    /// assert_eq!(slug.as_str(), "ueber-groesse");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`ParseSlugError::Empty`] when the title has no letters or
    /// digits to keep.
    pub fn from_title(
        title: &str,
        locale: &Locale,
        mode: SlugMode,
    ) -> Result<Self, ParseSlugError> {
        let words = match mode {
            SlugMode::Ascii => {
                let rules = if locale.language() == "de" {
                    Spelling::German
                } else {
                    Spelling::Latin
                };
                let ascii = ascii_words(title, rules);
                if ascii.is_empty() {
                    unicode_words(title)
                } else {
                    ascii
                }
            }
            SlugMode::Unicode => unicode_words(title),
        };
        let text = join_within_limit(&words);
        if text.is_ascii() {
            Self::try_new(&text)
        } else {
            Self::try_new_unicode(&text)
        }
    }
}

/// Which transliteration rules apply to a title.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Spelling {
    /// German: `ä ö ü` become `ae oe ue`.
    German,
    /// Every other Latin script: accents are dropped.
    Latin,
}

/// The ASCII spelling of letters that Unicode decomposition does not reach.
fn transliterate(c: char, rules: Spelling) -> Option<&'static str> {
    let german_only = match c {
        'ä' | 'Ä' => Some("ae"),
        'ö' | 'Ö' => Some("oe"),
        'ü' | 'Ü' => Some("ue"),
        _ => None,
    };
    if rules == Spelling::German && german_only.is_some() {
        return german_only;
    }
    match c {
        'ß' | 'ẞ' => Some("ss"),
        'æ' | 'Æ' => Some("ae"),
        'œ' | 'Œ' => Some("oe"),
        'ø' | 'Ø' => Some("o"),
        'ł' | 'Ł' => Some("l"),
        'đ' | 'Đ' => Some("d"),
        'þ' | 'Þ' => Some("th"),
        'ı' => Some("i"),
        '\'' | '\u{2019}' => Some(""),
        _ => None,
    }
}

/// The title's words in lowercase ASCII, or none when it has no letters
/// with an ASCII form.
fn ascii_words(title: &str, rules: Spelling) -> Vec<String> {
    let mut spelled = String::with_capacity(title.len());
    for c in title.chars() {
        match transliterate(c, rules) {
            Some(ascii) => spelled.push_str(ascii),
            None => spelled.push(c),
        }
    }
    let plain: String = spelled
        .nfkd()
        .filter(|&c| !is_combining_mark(c))
        .map(|c| c.to_ascii_lowercase())
        .collect();
    plain
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect()
}

/// The title's words in lowercase, in any script, in Unicode NFC form. A
/// capital with no lowercase form, such as a mathematical letter, is a break.
fn unicode_words(title: &str) -> Vec<String> {
    let lower: String = title
        .chars()
        .filter(|&c| transliterate(c, Spelling::Latin) != Some(""))
        .flat_map(char::to_lowercase)
        .collect();
    let lower: String = lower.nfc().collect();
    lower
        .split(|c: char| !c.is_alphanumeric() || c.is_uppercase())
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Joins words with hyphens, stopping before the word that would pass the
/// limit. A first word that is too long on its own is cut by characters.
fn join_within_limit(words: &[String]) -> String {
    let mut slug = String::new();
    for word in words {
        let separator = usize::from(!slug.is_empty());
        if encoded_len(&slug) + separator + encoded_len(word) > Slug::MAX_LEN {
            if slug.is_empty() {
                for c in word.chars() {
                    if encoded_len(&slug) + encoded_len(c.encode_utf8(&mut [0; 4])) > Slug::MAX_LEN
                    {
                        break;
                    }
                    slug.push(c);
                }
            }
            break;
        }
        if separator == 1 {
            slug.push('-');
        }
        slug.push_str(word);
    }
    slug
}
