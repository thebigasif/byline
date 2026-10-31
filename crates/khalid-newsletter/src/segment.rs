//! Segments and labels: which confirmed subscribers an email goes to.

use std::collections::BTreeSet;

use khalid_core::Slug;

use crate::{EmailAddress, SubscriberId};

/// The confirmed subscribers of one newsletter who carry every one of some
/// labels, such as everyone on `weekly` labelled `paid`. With no labels it
/// is the whole newsletter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    list: Slug,
    labels: BTreeSet<Slug>,
}

impl Segment {
    /// Everyone confirmed on `list` who carries all of `labels`.
    pub fn new(list: Slug, labels: impl IntoIterator<Item = Slug>) -> Self {
        Self {
            list,
            labels: labels.into_iter().collect(),
        }
    }

    /// The newsletter.
    pub fn list(&self) -> &Slug {
        &self.list
    }

    /// The labels every subscriber in the segment carries, in alphabetical
    /// order and each once.
    pub fn labels(&self) -> impl ExactSizeIterator<Item = &Slug> {
        self.labels.iter()
    }
}

/// One subscriber an email goes to.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Recipient {
    /// The subscriber's row.
    pub id: SubscriberId,
    /// Where the email goes.
    pub email: EmailAddress,
}

impl Recipient {
    /// Subscriber `id`, at `email`.
    pub fn new(id: SubscriberId, email: EmailAddress) -> Self {
        Self { id, email }
    }
}

/// What adding or taking away a label did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Labelled {
    /// The subscriber carries the label now, or no longer carries it.
    Done,
    /// No one subscribed has this address, so nothing changed.
    NotSubscribed,
}
