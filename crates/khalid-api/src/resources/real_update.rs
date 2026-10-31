//! The honest update switch on an edit (F2): the one way an edit moves a
//! live post's updated date.

use serde::Deserialize;
use utoipa::ToSchema;

/// The longest what-changed note, in characters: one short sentence, as
/// it shows beside the updated date on the page.
pub const MAX_UPDATE_NOTE: usize = 160;

/// An edit's claim that it is a real update: it changes what a live post
/// says, and `note` tells readers what. Only then does the post's
/// updated date become today. Title, description and settings edits
/// alone never move it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RealUpdate {
    /// What changed, in one short sentence, such as "Refreshed the 2024
    /// numbers." Shown beside the updated date. At most 160 characters.
    #[schema(value_type = String, example = "Refreshed the 2024 numbers.")]
    note: UpdateNote,
}

impl RealUpdate {
    /// What changed.
    pub fn note(&self) -> &str {
        self.note.0.as_str()
    }
}

/// Why an edit cannot be a real update. Nothing was saved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum NotRealUpdate {
    /// The post is a draft or scheduled, so readers have no date to
    /// update yet.
    #[error("the post is not live")]
    NotLive,
    /// The edit leaves the post's text as it was, so nothing a reader
    /// sees as new has changed.
    #[error("the edit leaves the post's text as it was")]
    TextUnchanged,
}

impl NotRealUpdate {
    /// Why it matters, in one plain sentence for the person who sent it.
    pub fn why(self) -> &'static str {
        match self {
            Self::NotLive => {
                "Only a live post has a date readers see, and this one has not gone live."
            }
            Self::TextUnchanged => {
                "A real update changes what the post says, and this edit leaves its text as it was."
            }
        }
    }

    /// What to do, in one plain sentence.
    pub fn fix(self) -> &'static str {
        match self {
            Self::NotLive => "Publish the post first, or leave real_update out.",
            Self::TextUnchanged => {
                "Change the post's text in the same edit, or leave real_update out."
            }
        }
    }
}

/// A what-changed note: not blank, trimmed, and at most
/// [`MAX_UPDATE_NOTE`] characters.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
struct UpdateNote(String);

impl TryFrom<String> for UpdateNote {
    type Error = &'static str;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        let note = text.trim();
        if note.is_empty() {
            return Err("real_update needs a note saying what changed");
        }
        if note.chars().count() > MAX_UPDATE_NOTE {
            return Err("real_update's note must be one short sentence of at most 160 characters");
        }
        Ok(Self(note.to_owned()))
    }
}
