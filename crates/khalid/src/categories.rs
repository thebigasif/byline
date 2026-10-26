//! The site's categories, from `categories.toml` in the site folder.

use std::collections::BTreeMap;
use std::path::Path;

use khalid_core::{Locale, Slug, UserMessage};
use khalid_db::IndexedTerm;
use serde::Deserialize;

/// The file's name in the site folder.
const FILE: &str = "categories.toml";

/// Every categories message shares this reason.
const WHY: &str = "categories.toml names the groups posts belong to, so Khalid reads it first.";

/// The file: a list of categories.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    #[serde(default)]
    categories: Vec<Entry>,
}

/// One category: its slug and its name in each language, by language code.
#[derive(Deserialize)]
struct Entry {
    slug: Option<String>,
    #[serde(flatten)]
    names: BTreeMap<String, toml::Value>,
}

/// Every category in `site`'s `categories.toml`, in the file's order, once
/// per language it is named in, by language code. A site without the file
/// has none.
pub(crate) fn of(site: &Path) -> Result<Vec<IndexedTerm>, UserMessage> {
    let mut path = site.to_path_buf();
    path.push(FILE);
    let at = path.display().to_string();
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(_) => return Err(unreadable(&at)),
    };
    let file: File = toml::from_str(&text).map_err(|error| {
        let line = error
            .span()
            .and_then(|span| text.get(..span.start))
            .map_or(1, |before| before.matches('\n').count().saturating_add(1));
        wrong(&at, line, error.message())
    })?;
    let mut terms = Vec::new();
    for (number, category) in file.categories.iter().enumerate() {
        let entry = number.saturating_add(1);
        let slug = category
            .slug
            .as_deref()
            .and_then(|slug| Slug::try_new(slug).ok())
            .ok_or_else(|| no_slug(&at, entry))?;
        for (key, name) in &category.names {
            let locale = Locale::try_new(key).map_err(|_| not_a_language(&at, entry, key))?;
            let name = name.as_str().ok_or_else(|| not_text(&at, entry, key))?;
            terms.push(IndexedTerm::new(locale, slug.clone(), name));
        }
    }
    Ok(terms)
}

fn not_text(at: &str, entry: usize, key: &str) -> UserMessage {
    UserMessage::new(
        format!("Category {entry} in {at} gives `{key}` a value that is not text."),
        WHY,
        format!("Write the name in quotes, such as {key} = \"On-page SEO\"."),
    )
}

fn unreadable(at: &str) -> UserMessage {
    UserMessage::new(
        format!("Khalid could not read {at}."),
        WHY,
        format!("Check that {at} is a file you can open, or remove it."),
    )
}

fn wrong(at: &str, line: usize, detail: &str) -> UserMessage {
    UserMessage::new(
        format!("{at}, line {line}: {}.", detail.trim_end_matches('.')),
        WHY,
        "Write each category as a [[categories]] table with a slug and a name per language.",
    )
}

fn no_slug(at: &str, entry: usize) -> UserMessage {
    UserMessage::new(
        format!("Category {entry} in {at} has no usable slug."),
        WHY,
        "Give it a slug of lowercase letters, digits and hyphens, such as slug = \"on-page-seo\".",
    )
}

fn not_a_language(at: &str, entry: usize, key: &str) -> UserMessage {
    UserMessage::new(
        format!("Category {entry} in {at} has `{key}`, which is not a language code."),
        WHY,
        format!("Rename `{key}` to a language code such as en or de, or remove it."),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn site_with(text: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let mut path = dir.path().to_path_buf();
        path.push(FILE);
        std::fs::write(path, text).unwrap();
        dir
    }

    #[test]
    fn each_category_is_named_once_per_language_in_the_file_order() {
        let dir = site_with(
            "[[categories]]\nslug = \"links\"\nen = \"Links\"\nde = \"Verlinkung\"\n\n\
             [[categories]]\nslug = \"speed\"\nen = \"Speed\"\n",
        );
        let terms = of(dir.path()).unwrap();
        let named: Vec<String> = terms
            .iter()
            .map(|t| format!("{}.{} {}", t.locale.as_str(), t.slug.as_str(), t.name))
            .collect();
        assert_eq!(
            named,
            ["de.links Verlinkung", "en.links Links", "en.speed Speed"]
        );
    }

    #[test]
    fn a_site_without_the_file_has_no_categories() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(of(dir.path()).unwrap(), []);
    }

    #[test]
    fn each_mistake_is_explained_with_where_it_is() {
        let cases = [
            ("[[categories]]\nslug = \"links\"\nen = 3\n", "`en` a value"),
            ("[[categories]]\nslug = \"links\"\nen = \n", "line 3"),
            ("[[categories]]\nen = \"Links\"\n", "Category 1"),
            ("[[categories]]\nslug = \"Links!\"\n", "no usable slug"),
            (
                "[[categories]]\nslug = \"links\"\nenglish = \"Links\"\n",
                "`english`",
            ),
            ("[tags]\n", "line 1"),
        ];
        for (text, want) in cases {
            let dir = site_with(text);
            let message = of(dir.path()).unwrap_err();
            let shown = format!("{} {} {}", message.what(), message.why(), message.fix());
            assert!(shown.contains(want), "{text:?}: {shown}");
            assert!(message.what().ends_with('.'), "{shown}");
        }
    }
}
