//! What the Admin API's routes for pages, snippets and authors share: the
//! store a site may not have, and the problems an entry store's errors
//! answer with.

use std::sync::Arc;

use axum::Json;
use axum::extract::rejection::JsonRejection;
use chrono::{DateTime, Utc};
use khalid_core::UserMessage;

use crate::entry_store::{EntryError, EntryStore};
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
    /// An identity the messages give as an example, such as `en.about`.
    pub(crate) example: &'static str,
}

impl Noun {
    /// The store, or a 404 when this site keeps none of these here.
    pub(crate) fn store<S: ?Sized>(self, kept: Option<&S>) -> Result<&S, Problem> {
        kept.ok_or_else(|| {
            let message = UserMessage::new(
                format!("This server keeps no {} through the Admin API.", self.many),
                "It was started without the site's files, as some tests and tools are.",
                format!(
                    "Send it to the running khalid serve, which reads each {} from the site's files.",
                    self.one
                ),
            );
            Problem::new(ProblemType::NotFound, &message)
        })
    }

    /// The body of a request, or a 422 saying why it could not be read.
    pub(crate) fn body<T>(self, body: Result<Json<T>, JsonRejection>) -> Result<T, Problem> {
        let Json(body) = body.map_err(|rejection| self.invalid(&rejection.body_text(), None))?;
        Ok(body)
    }

    /// The one item of a request's list, or a 422.
    pub(crate) fn one<T>(self, items: Vec<T>) -> Result<T, Problem> {
        let [item] = <[T; 1]>::try_from(items).map_err(|_| {
            let reason = format!("{} must hold exactly one {}", self.many, self.one);
            self.invalid(&reason, None)
        })?;
        Ok(item)
    }

    /// The problem `error` answers with, for the entry at `path`.
    pub(crate) fn problem(self, error: EntryError, path: &str) -> Problem {
        match error {
            EntryError::NotFound => self.missing(path),
            EntryError::Stale { updated_at } => self.stale(updated_at),
            EntryError::Taken(id) => {
                let message = UserMessage::new(
                    format!("Another {} already has the identity {id}.", self.one),
                    "Two entries in one language cannot share an address.",
                    "Send a different slug, or edit the one that is there.",
                );
                Problem::new(ProblemType::Conflict, &message).with_rule("slug")
            }
            EntryError::Unsupported { field, why } => {
                let message = UserMessage::new(
                    format!("This site cannot keep this {}.", self.one),
                    why,
                    format!("Change {field} and send it again; nothing was saved."),
                );
                Problem::new(ProblemType::InvalidBody, &message).with_rule(field)
            }
            error => Problem::internal(&error),
        }
    }

    /// The 422 for a body that cannot be kept, naming `field` when known.
    pub(crate) fn invalid(self, reason: &str, field: Option<&str>) -> Problem {
        let message = UserMessage::new(
            format!("The request body could not be read: {reason}."),
            "Nothing was changed.",
            format!(
                "Send {} shaped like the example in /api/openapi.json.",
                self.many
            ),
        );
        let problem = Problem::new(ProblemType::InvalidBody, &message);
        match field {
            Some(field) => problem.with_rule(field),
            None => problem,
        }
    }

    /// The 404 for an identity no entry has.
    pub(crate) fn missing(self, path: &str) -> Problem {
        let message = UserMessage::new(
            format!("No {} has this identity.", self.one),
            "It may have been deleted, or the identity may be mistyped.",
            format!(
                "Check the identity, such as {}, or list the {}.",
                self.example, self.many
            ),
        );
        Problem::new(ProblemType::NotFound, &message).with_instance(path)
    }

    /// Refuses a `field` sent empty, such as a title of only spaces.
    pub(crate) fn filled(self, field: &'static str, value: &str) -> Result<(), Problem> {
        if value.trim().is_empty() {
            let reason = format!("{field} must not be empty");
            return Err(self.invalid(&reason, Some(field)));
        }
        Ok(())
    }

    /// The 400 for a delete that does not say which copy it was made on.
    pub(crate) fn no_copy(self) -> Problem {
        let message = UserMessage::new(
            format!(
                "This delete does not say which copy of the {} it was made on.",
                self.one
            ),
            format!(
                "Without it, a {} someone changed a moment ago could be deleted unseen.",
                self.one
            ),
            format!(
                "Add updated_at with the time the {} last changed, such as ?updated_at=2026-10-20T08:30:00Z.",
                self.one
            ),
        );
        Problem::new(ProblemType::InvalidQuery, &message).with_rule("updated_at")
    }

    fn stale(self, updated_at: DateTime<Utc>) -> Problem {
        let when = updated_at.format("%Y-%m-%d %H:%M UTC");
        let message = UserMessage::new(
            format!(
                "This {} changed at {when}, after the copy you have.",
                self.one
            ),
            "Saving your copy would undo that change.",
            format!(
                "Fetch the {} again, make your change to it, and send its new updated_at.",
                self.one
            ),
        );
        Problem::new(ProblemType::Conflict, &message).with_rule("updated_at")
    }
}
