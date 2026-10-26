//! The site's settings a front end needs.

use khalid_core::{AbsoluteUrl, Locale};
use serde::{Serialize, Serializer};
use utoipa::ToSchema;

use super::display;

/// The public settings of the site, in API shape: its name, where it lives
/// and its languages. Nothing secret is ever part of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct Settings {
    /// The site's name.
    title: String,
    /// The address of the blog's front page in the default language.
    #[serde(serialize_with = "display")]
    #[schema(value_type = String, example = "https://example.com/blog/")]
    url: AbsoluteUrl,
    /// The language a reader gets first, such as `en`.
    #[serde(serialize_with = "display")]
    #[schema(value_type = String, example = "en")]
    default_language: Locale,
    /// Every language the site is written in, the default among them.
    #[serde(serialize_with = "locales")]
    #[schema(value_type = Vec<String>, example = json!(["en", "de"]))]
    languages: Vec<Locale>,
}

impl Settings {
    /// The settings of the site `title` at `url`. `default_language` is
    /// added at the front of `languages` when the list leaves it out.
    pub fn new(
        title: &str,
        url: AbsoluteUrl,
        default_language: Locale,
        languages: Vec<Locale>,
    ) -> Self {
        let mut languages = languages;
        if !languages.contains(&default_language) {
            languages.insert(0, default_language.clone());
        }
        Self {
            title: title.to_owned(),
            url,
            default_language,
            languages,
        }
    }
}

/// Writes each language through its `Display`.
fn locales<S: Serializer>(languages: &[Locale], serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(languages.iter().map(Locale::as_str))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_language_is_always_among_the_languages() {
        let locale = |code: &str| Locale::try_new(code).unwrap();
        let url = AbsoluteUrl::parse("https://example.com/blog/").unwrap();
        let settings = Settings::new("Notes", url, locale("de"), vec![locale("en")]);
        assert_eq!(settings.languages, [locale("de"), locale("en")]);
        let kept = Settings::new("Notes", settings.url, locale("en"), settings.languages);
        assert_eq!(kept.languages, [locale("de"), locale("en")]);
    }
}
