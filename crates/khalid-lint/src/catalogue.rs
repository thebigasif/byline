//! The messages every rule shows, read from `catalogue.toml`.

use std::collections::BTreeMap;
use std::collections::btree_map;

use khalid_core::{Slug, UserMessage};
use serde::Deserialize;

use crate::LintError;

/// The catalogue file, built into the crate so every user shows the same words.
const BUILTIN: &str = include_str!("../catalogue.toml");

/// A lint message: what is wrong, why it matters, what to do, and a guide.
///
/// Each part is one plain sentence for a writer who is new to SEO
/// (RULES.md BL-TXT-2). `guide` names the `SerpDad` guide that explains the
/// rule at length; the docs map it to an address later.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Message {
    what: String,
    why: String,
    fix: String,
    guide: Slug,
}

impl Message {
    /// What is wrong.
    pub fn what(&self) -> &str {
        &self.what
    }

    /// Why it matters to the writer.
    pub fn why(&self) -> &str {
        &self.why
    }

    /// What the writer does to fix it.
    pub fn fix(&self) -> &str {
        &self.fix
    }

    /// The guide that explains the rule at length.
    pub fn guide(&self) -> &Slug {
        &self.guide
    }

    /// The three sentences as the message every Khalid surface shows.
    pub fn to_user_message(&self) -> UserMessage {
        UserMessage::new(self.what.clone(), self.why.clone(), self.fix.clone())
    }
}

/// Every message, keyed by rule ID, in rule ID order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Catalogue(BTreeMap<String, Message>);

impl Catalogue {
    /// Reads the catalogue built into this crate.
    ///
    /// # Errors
    ///
    /// Returns [`LintError::Catalogue`] if the built-in file is malformed,
    /// which the crate's own tests rule out.
    pub fn builtin() -> Result<Self, LintError> {
        toml::from_str(BUILTIN)
            .map(Self)
            .map_err(|source| LintError::Catalogue { source })
    }

    /// The message for `rule`, if the catalogue has one.
    pub fn get(&self, rule: &str) -> Option<&Message> {
        self.0.get(rule)
    }

    /// Whether the catalogue has a message for `rule`.
    pub fn contains(&self, rule: &str) -> bool {
        self.0.contains_key(rule)
    }

    /// Every rule ID with its message, in rule ID order.
    pub fn iter(&self) -> btree_map::Iter<'_, String, Message> {
        self.0.iter()
    }

    /// How many messages the catalogue holds.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the catalogue holds no messages.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<'a> IntoIterator for &'a Catalogue {
    type Item = (&'a String, &'a Message);
    type IntoIter = btree_map::Iter<'a, String, Message>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
