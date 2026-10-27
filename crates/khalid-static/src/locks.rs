//! The notes that stand in for forms, one file per language, and the page
//! rewrite that puts them in.

use std::collections::{BTreeMap, BTreeSet};

use khalid_core::Locale;
use serde::Deserialize;

use crate::{FormKind, LockError, ServerFeature, forms};

/// The language whose notes fill in for a language with no file.
const FALLBACK_LANGUAGE: &str = "en";

/// The built-in notes files, by language.
const FILES: [(&str, &str); 2] = [
    ("en", include_str!("../strings/en.toml")),
    ("de", include_str!("../strings/de.toml")),
];

/// One language's notes, one sentence for each kind of form.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Notes {
    comments: String,
    newsletter: String,
    forms: String,
    search: String,
}

impl Notes {
    fn of(&self, kind: FormKind) -> &str {
        match kind {
            FormKind::Comments => &self.comments,
            FormKind::Newsletter => &self.newsletter,
            FormKind::Forms => &self.forms,
            FormKind::Search => &self.search,
        }
    }
}

/// The notes in every built-in language, ready to lock pages.
#[derive(Debug, Clone)]
pub struct Locks {
    notes: BTreeMap<&'static str, Notes>,
}

/// A page after locking: its HTML, and the features its forms needed.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LockedPage {
    /// The page with each of those forms replaced by its note.
    pub html: String,
    /// The features the replaced forms needed, in [`ServerFeature`] order.
    pub locked: BTreeSet<ServerFeature>,
}

impl Locks {
    /// Reads every built-in notes file.
    ///
    /// # Errors
    ///
    /// [`LockError::Notes`] when a file is not valid, which this crate's
    /// tests rule out.
    pub fn load() -> Result<Self, LockError> {
        let mut notes = BTreeMap::new();
        for (language, text) in FILES {
            let read = toml::from_str(text).map_err(|source| LockError::Notes {
                language: language.to_owned(),
                source,
            })?;
            notes.insert(language, read);
        }
        Ok(Self { notes })
    }

    /// `html`, a page in `locale`, with every form that sends to the site
    /// itself replaced by a note in the page's language, or in English when
    /// the language has no file. A form that sends to another site stays.
    ///
    /// # Errors
    ///
    /// [`LockError::Unclosed`] when a form has no closing tag, so where it
    /// ends is unknown.
    pub fn page(&self, html: &str, locale: &Locale) -> Result<LockedPage, LockError> {
        let notes = self
            .notes
            .get(locale.language())
            .or_else(|| self.notes.get(FALLBACK_LANGUAGE));
        let mut written = String::with_capacity(html.len());
        let mut locked = BTreeSet::new();
        let mut copied = 0;
        for form in forms(html)? {
            let Some(kind) = form.kind else {
                continue;
            };
            written.push_str(html.get(copied..form.start).unwrap_or_default());
            let note = notes.map_or("", |notes| notes.of(kind));
            written.push_str("<p class=\"locked\" data-feature=\"");
            written.push_str(kind.feature().name());
            written.push_str("\">");
            written.push_str(&escape(note));
            written.push_str("</p>");
            copied = form.end;
            locked.insert(kind.feature());
        }
        written.push_str(html.get(copied..).unwrap_or_default());
        Ok(LockedPage {
            html: written,
            locked,
        })
    }
}

/// `text` safe to put between HTML tags.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_built_in_file_reads() {
        let locks = Locks::load().unwrap();
        let languages: Vec<&str> = locks.notes.keys().copied().collect();
        assert_eq!(languages, ["de", "en"]);
    }

    #[test]
    fn every_note_is_one_short_sentence() {
        let locks = Locks::load().unwrap();
        for (language, notes) in &locks.notes {
            for lock in [
                FormKind::Comments,
                FormKind::Newsletter,
                FormKind::Forms,
                FormKind::Search,
            ] {
                let note = notes.of(lock);
                assert!(note.ends_with('.'), "{language} {lock:?}: {note}");
                assert_eq!(note.matches(". ").count(), 0, "{language} {lock:?}: {note}");
                assert!(note.len() < 80, "{language} {lock:?}: {note}");
            }
        }
    }
}
