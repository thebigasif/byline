//! Welcome emails (task 5.9): what a newsletter sends a reader the moment
//! they confirm.

use std::collections::BTreeMap;

use khalid_core::Slug;

use crate::PostalAddress;

/// One newsletter's welcome email, as its writer wrote it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Welcome {
    subject: String,
    text: String,
}

impl Welcome {
    /// Reads `markdown`, whose first line is a `# ` heading that becomes
    /// the subject and whose rest is the text. `None` when the first line
    /// is not such a heading or the text is empty.
    pub fn read(markdown: &str) -> Option<Self> {
        let markdown = markdown.trim_start_matches('\u{feff}').trim_start();
        let (first, rest) = markdown.split_once('\n').unwrap_or((markdown, ""));
        let subject = first.strip_prefix("# ")?.trim();
        let text = rest.trim();
        (!subject.is_empty() && !text.is_empty()).then(|| Self {
            subject: subject.to_owned(),
            text: format!("{text}\n"),
        })
    }

    /// The subject line.
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// The text, before the footer every newsletter email gets.
    pub fn text(&self) -> &str {
        &self.text
    }
}

/// The welcome emails of a site's newsletters, and the postal address they
/// end with: no welcome goes out without one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Welcomes {
    by_list: BTreeMap<Slug, Welcome>,
    postal: PostalAddress,
}

impl Welcomes {
    /// The welcome emails in `by_list`, each ending with `postal`.
    pub fn new(by_list: BTreeMap<Slug, Welcome>, postal: PostalAddress) -> Self {
        Self { by_list, postal }
    }

    /// The welcome email of `list`, if it has one.
    pub fn of(&self, list: &Slug) -> Option<&Welcome> {
        self.by_list.get(list)
    }

    /// The postal address they end with.
    pub fn postal(&self) -> &PostalAddress {
        &self.postal
    }
}
