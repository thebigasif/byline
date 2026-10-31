//! What the Admin API's routes for pages, snippets and authors share: the
//! store a site may not have, and the problems an entry store's errors
//! answer with.

use std::sync::Arc;

use khalid_core::UserMessage;

use crate::entry_store::{Entry, EntryStore};
use crate::problem::{Problem, ProblemType};

/// The store of one kind of entry, when the site keeps one.
pub(crate) type Kept<T> = Option<Arc<dyn EntryStore<T>>>;

/// How the messages name one kind of entry.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Noun {
    /// One, such as `page`.
    pub(crate) one: &'static str,
    /// Many, such as `pages`, as in the routes.
    pub(crate) many: &'static str,
}

impl Noun {
    /// The store, or a 404 when this site keeps none of these here.
    pub(crate) fn store<T: Entry>(
        self,
        kept: Option<&dyn EntryStore<T>>,
    ) -> Result<&dyn EntryStore<T>, Problem> {
        kept.ok_or_else(|| {
            let message = UserMessage::new(
                format!("This server keeps no {} through the Admin API.", self.many),
                "It was started without the site's files, as some tests and tools are.",
                format!(
                    "Send it to the running khalid serve, which keeps each {} as a file.",
                    self.one
                ),
            );
            Problem::new(ProblemType::NotFound, &message)
        })
    }
}
