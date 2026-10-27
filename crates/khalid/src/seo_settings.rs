//! The site-wide SEO tables of `site.toml`, checked with `khalid-seo`'s own
//! rules and applied to the site every page is made from (audit G11, ADR
//! "SEO settings: core reads their shape, khalid-seo checks their values
//! at start"). This module holds the shared mistake and `[seo.titles]`.

use std::fmt;

use khalid_core::toml_location::line_of_key;
use khalid_core::{SeoSettings, TitleSettings, UserMessage};
use khalid_seo::{MetaTemplate, PageKind, SeoSite, TemplatePair};

use crate::{config_message, seo_head_settings};

/// What a title or description template may use, for the fix.
const TEMPLATE: &str =
    "a template using {title}, {site}, {tagline}, {sep}, {excerpt}, {category} or {author}";

/// A value in an `[seo]` table that `khalid-seo` refuses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SettingMistake {
    /// The table it sits in, such as `seo.social`.
    pub(crate) table: String,
    /// Which `[[table]]` entry it sits in, counting from 0. Always 0 for a
    /// table written once.
    pub(crate) entry: usize,
    /// Its key.
    pub(crate) key: String,
    /// What is wrong, then `, expected ` and what is allowed when that helps.
    pub(crate) problem: String,
}

impl SettingMistake {
    /// The mistake on `key` in `table`: `problem`, then what is `expected`.
    pub(crate) fn new(
        table: &str,
        key: &str,
        problem: &dyn fmt::Display,
        expected: Option<&str>,
    ) -> Self {
        let problem = match expected {
            Some(expected) => format!("{problem}, expected {expected}"),
            None => problem.to_string(),
        };
        Self {
            table: table.to_owned(),
            entry: 0,
            key: key.to_owned(),
            problem,
        }
    }

    /// What a person reads when the mistake is found with no `site.toml`
    /// text at hand to point at its line.
    pub(crate) fn message(&self) -> UserMessage {
        let detail = self
            .problem
            .split_once(", expected ")
            .map_or(self.problem.as_str(), |(detail, _)| detail);
        UserMessage::new(
            format!("site.toml, [{}], `{}`: {detail}.", self.table, self.key),
            config_message::WHY,
            format!(
                "Correct `{}` under [{}] in site.toml, then run the command again.",
                self.key, self.table
            ),
        )
    }

    /// The 1-based line of `text`, the `site.toml` the settings came from,
    /// where the key is written: below its table's header, so `name` under
    /// `[seo.publisher]` is not taken for the site's `name`. Falls back to
    /// the first line that sets the key anywhere.
    pub(crate) fn line_in(&self, text: &str) -> usize {
        let mut inside = false;
        let mut seen = 0_usize;
        for (index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') {
                let name = trimmed.trim_matches(['[', ']']).trim();
                inside = name == self.table && seen == self.entry;
                if name == self.table {
                    seen = seen.saturating_add(1);
                }
                continue;
            }
            let here = line
                .split_once('=')
                .is_some_and(|(left, _)| left.trim().trim_matches('"') == self.key);
            if inside && here {
                return index.saturating_add(1);
            }
        }
        line_of_key(text, &self.key)
    }
}

/// Checks each value in `settings` and sets it on `site`.
///
/// # Errors
///
/// The first value `khalid-seo` refuses, with its table and key.
pub(crate) fn apply(settings: &SeoSettings, site: &mut SeoSite) -> Result<(), SettingMistake> {
    titles(&settings.titles, site)?;
    seo_head_settings::social(&settings.social, site)?;
    seo_head_settings::verification(&settings.verification, site)?;
    Ok(())
}

fn titles(settings: &TitleSettings, site: &mut SeoSite) -> Result<(), SettingMistake> {
    if let Some(separator) = &settings.separator {
        site.separator.clone_from(separator);
    }
    site.tagline.clone_from(&settings.tagline);
    let kinds = [
        (PageKind::Home, "home", &settings.home),
        (PageKind::Post, "post", &settings.post),
        (PageKind::Page, "page", &settings.page),
        (PageKind::Author, "author", &settings.author),
        (PageKind::Tag, "tag", &settings.tag),
        (PageKind::Category, "category", &settings.category),
        (PageKind::Collection, "collection", &settings.collection),
        (PageKind::Date, "date", &settings.date),
        (PageKind::Search, "search", &settings.search),
        (PageKind::GlossaryIndex, "glossary", &settings.glossary),
        (
            PageKind::GlossaryTerm,
            "glossary_term",
            &settings.glossary_term,
        ),
        (PageKind::NotFound, "not_found", &settings.not_found),
    ];
    for (kind, name, pair) in kinds {
        let Some(pair) = pair else { continue };
        let table = format!("seo.titles.{name}");
        let current = site.templates.for_kind(kind).clone();
        let title = template(&table, "title", pair.title.as_deref())?;
        let description = template(&table, "description", pair.description.as_deref())?;
        site.templates.set(
            kind,
            TemplatePair::new(
                title.unwrap_or(current.title),
                description.unwrap_or(current.description),
            ),
        );
    }
    Ok(())
}

fn template(
    table: &str,
    key: &str,
    text: Option<&str>,
) -> Result<Option<MetaTemplate>, SettingMistake> {
    text.map(MetaTemplate::parse)
        .transpose()
        .map_err(|error| SettingMistake::new(table, key, &error, Some(TEMPLATE)))
}

/// A bare site and settings read from `[seo]` tables, for the tests of
/// each table.
#[cfg(test)]
pub(crate) mod testing {
    use std::path::Path;

    use khalid_core::{AbsoluteUrl, Config, Environment, Locale, SeoSettings};
    use khalid_seo::SeoSite;

    /// A site with nothing set.
    pub(crate) fn site() -> SeoSite {
        let address = AbsoluteUrl::parse("https://www.example.com").unwrap();
        SeoSite::new("Field Notes", address, Locale::try_new("en").unwrap())
    }

    /// The `[seo]` settings of a site whose `site.toml` ends in `tables`.
    pub(crate) fn settings(tables: &str) -> SeoSettings {
        let text = format!(
            "name = \"a\"\naddress = \"https://www.example.com\"\ndefault_language = \"en\"\nlanguages = [\"en\"]\n{tables}"
        );
        Config::from_toml(Path::new("site.toml"), &text, &Environment::empty())
            .unwrap()
            .seo()
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::testing::{settings, site};
    use super::*;

    fn mistake(table: &str, entry: usize, key: &str) -> SettingMistake {
        SettingMistake {
            table: table.to_owned(),
            entry,
            key: key.to_owned(),
            problem: String::new(),
        }
    }

    #[test]
    fn a_key_is_found_below_its_own_table() {
        let text =
            "name = \"Field Notes\"\n\n[seo.publisher]\ntype = \"person\"\nname = \"Asif\"\n";
        assert_eq!(mistake("seo.publisher", 0, "name").line_in(text), 5);
        assert_eq!(mistake("seo.social", 0, "name").line_in(text), 1);
    }

    #[test]
    fn a_key_is_found_in_the_right_entry_of_an_array_of_tables() {
        let text = "[[seo.schema]]\ntype = \"Course\"\n[[seo.schema]]\ntype = \"Recipe\"\n";
        assert_eq!(mistake("seo.schema", 1, "type").line_in(text), 4);
    }

    #[test]
    fn a_quoted_key_is_found() {
        let text = "[seo.verification.files]\n\"robots.txt\" = \"x\"\n";
        assert_eq!(
            mistake("seo.verification.files", 0, "robots.txt").line_in(text),
            2
        );
    }

    #[test]
    fn a_title_alone_keeps_the_default_description() {
        let mut site = site();
        apply(
            &settings("[seo.titles.tag]\ntitle = \"#{title}\"\n"),
            &mut site,
        )
        .unwrap();
        let tag = site.templates.for_kind(PageKind::Tag);
        assert_eq!(
            (tag.title.as_str(), tag.description.as_str()),
            ("#{title}", "{excerpt}")
        );
    }

    #[test]
    fn a_mistake_found_without_the_file_names_its_table() {
        let error = apply(
            &settings("[seo.titles.home]\ntitle = \"{site\"\n"),
            &mut site(),
        )
        .unwrap_err();
        let message = error.message();
        assert_eq!(
            message.what(),
            "site.toml, [seo.titles.home], `title`: the template \"{site\" has a brace without its partner."
        );
    }
}
