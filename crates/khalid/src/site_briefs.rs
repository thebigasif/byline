//! The writer's brief for each post, from `briefs/` in the site folder (P6).
//!
//! A brief is optional. It sits at `briefs/<language>/<slug>.toml`, named
//! after the post's language folder and the last part of its address:
//!
//! ```toml
//! questions = ["How many internal links should a post have?"]
//! terms = ["anchor text"]
//! links = ["/blog/choosing-link-text/"]
//! notes = "For readers new to SEO."
//! ```
//!
//! The linter warns about each question no heading asks, each term the post
//! never uses and each page it does not link to. `notes` is for the writer
//! and is never checked.

use std::collections::BTreeMap;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use khalid_core::{Locale, SitePath, Slug, UserMessage};
use khalid_lint::LintBrief;
use serde::Deserialize;

/// Each post's brief, by the post's language and slug.
pub(crate) type Briefs = BTreeMap<(Locale, Slug), LintBrief>;

/// The folder of briefs in the site folder.
const FOLDER: &str = "briefs";

/// Every brief message shares this reason.
const WHY: &str = "Khalid checks each post against its brief, so it reads every brief first.";

/// One brief as written.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BriefFile {
    #[serde(default)]
    questions: Vec<String>,
    #[serde(default)]
    terms: Vec<String>,
    #[serde(default)]
    links: Vec<String>,
    #[serde(default, rename = "notes")]
    _notes: Option<String>,
}

/// Every brief in `site`'s `briefs/` folder. A site without the folder has
/// none. Files that do not end in `.toml`, and names that start with a
/// dot, are left alone, so a folder may hold a README.
///
/// # Errors
///
/// Returns what, why and fix when a brief cannot be read or is not a
/// brief, when a `.toml` file sits outside a language folder, or when a
/// folder or file name is not a language or an address.
pub(crate) fn load(site: &Path) -> Result<Briefs, UserMessage> {
    let mut briefs = Briefs::new();
    for (language, is_folder) in names(site, FOLDER)? {
        let shown = format!("{FOLDER}/{language}");
        if !is_folder {
            if toml_stem(&language).is_some() {
                return Err(outside_language(&shown));
            }
            continue;
        }
        let locale = Locale::try_new(&language).map_err(|_| bad_language(&shown))?;
        for (name, is_folder) in names(site, &shown)? {
            let Some(stem) = toml_stem(&name).filter(|_| !is_folder) else {
                continue;
            };
            let file = format!("{shown}/{name}");
            let slug = Slug::try_new(stem)
                .or_else(|_| Slug::try_new_unicode(stem))
                .map_err(|_| bad_name(&file))?;
            briefs.insert((locale.clone(), slug), read(site, &file)?);
        }
    }
    Ok(briefs)
}

/// The file name `name` without its `.toml` ending, in any letter case, or
/// `None` when it has another ending.
fn toml_stem(name: &str) -> Option<&str> {
    let (stem, ending) = name.rsplit_once('.')?;
    ending.eq_ignore_ascii_case("toml").then_some(stem)
}

/// The names in the folder `shown` inside `site`, sorted, each with whether
/// it is a folder. Names starting with a dot are left out. A missing folder
/// has none.
fn names(site: &Path, shown: &str) -> Result<Vec<(String, bool)>, UserMessage> {
    let entries = match fs::read_dir(inside(site, shown)?) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(_) => return Err(unreadable(shown)),
    };
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|_| unreadable(shown))?;
        let Ok(name) = entry.file_name().into_string() else {
            return Err(unreadable(shown));
        };
        if name.starts_with('.') {
            continue;
        }
        let is_folder = entry.file_type().map_err(|_| unreadable(shown))?.is_dir();
        names.push((name, is_folder));
    }
    names.sort();
    Ok(names)
}

/// The real path of `shown` inside `site`, refused when a symlink leads
/// out of the site folder.
fn inside(site: &Path, shown: &str) -> Result<PathBuf, UserMessage> {
    let path = SitePath::within(site, shown).map_err(|_| unreadable(shown))?;
    path.resolve().map_err(|_| unreadable(shown))
}

/// The brief in the file `shown` inside `site`.
fn read(site: &Path, shown: &str) -> Result<LintBrief, UserMessage> {
    let text = fs::read_to_string(inside(site, shown)?).map_err(|_| unreadable(shown))?;
    parse(shown, &text)
}

/// The brief in `text`, the contents of the file `shown`.
fn parse(shown: &str, text: &str) -> Result<LintBrief, UserMessage> {
    let file: BriefFile = toml::from_str(text).map_err(|error| {
        let line = error
            .span()
            .and_then(|span| text.get(..span.start))
            .map_or(1, |before| before.matches('\n').count().saturating_add(1));
        wrong(shown, line, error.message())
    })?;
    Ok(LintBrief::new(file.questions, file.terms, file.links))
}

fn unreadable(shown: &str) -> UserMessage {
    UserMessage::new(
        format!("Khalid could not read {shown} in the site folder."),
        WHY,
        format!("Check that {shown} is something you can open, or remove it."),
    )
}

fn wrong(shown: &str, line: usize, detail: &str) -> UserMessage {
    UserMessage::new(
        format!(
            "{shown} has a mistake on line {line}: {}.",
            detail.trim_end_matches('.')
        ),
        WHY,
        "Write a brief as lists of text named questions, terms and links, and notes as one text.",
    )
}

fn outside_language(shown: &str) -> UserMessage {
    UserMessage::new(
        format!("The brief {shown} is not in a language folder."),
        WHY,
        "Move it into the folder of its post's language, such as briefs/en/.",
    )
}

fn bad_language(shown: &str) -> UserMessage {
    UserMessage::new(
        format!("The folder {shown} is not named after a language."),
        WHY,
        "Rename it to the language code its posts use, such as en or de.",
    )
}

fn bad_name(shown: &str) -> UserMessage {
    UserMessage::new(
        format!("The brief {shown} is not named after a post's address."),
        WHY,
        "Rename it to the last part of its post's address, such as why-internal-links-matter.toml.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_brief_reads_its_three_lists_and_keeps_notes_to_itself() {
        let text = "questions = [\"Why?\"]\nterms = [\"anchor text\"]\nlinks = [\"/blog/a/\"]\nnotes = \"For new writers.\"\n";
        let brief = parse("briefs/en/a.toml", text).unwrap();
        let want = LintBrief::new(
            vec!["Why?".to_owned()],
            vec!["anchor text".to_owned()],
            vec!["/blog/a/".to_owned()],
        );
        assert_eq!(brief, want);
    }
}
