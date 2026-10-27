//! The comment form under a post, as the browser posts it.

use serde::Deserialize;

use crate::{AuthorName, CommentAuthor, CommentId, CommentText, FieldProblem, ReaderEmail};

/// The fields of the comment form, as `application/x-www-form-urlencoded`
/// sends them. A field the browser leaves out reads as empty.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct CommentForm {
    /// The address of the post's page, such as `/blog/why-links-matter/`.
    pub page: String,
    /// The comment answered, by its identity, when this is a reply.
    pub parent: Option<String>,
    /// The reader's name.
    pub name: String,
    /// The reader's email address, which may be empty.
    pub email: String,
    /// What the reader wrote.
    pub comment: String,
    /// The honeypot: a field people never see, so only a bot fills it in.
    pub website: String,
}

/// What a posted form turned out to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reading {
    /// A comment whose every field is valid.
    Comment(FormComment),
    /// A bot, since it filled in the honeypot. Answer it as if the
    /// comment were kept, so it learns nothing, and keep nothing.
    Bait,
    /// What is wrong with the fields, in the order the form shows them.
    Problems(Vec<FieldProblem>),
}

/// A comment read from the form, not yet tied to a post or stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormComment {
    /// The address of the page it was sent from, not yet checked.
    pub page: String,
    /// The comment answered, not yet checked.
    pub parent: Option<CommentId>,
    /// The reader, with their name and email address.
    pub author: CommentAuthor,
    /// What they wrote.
    pub text: CommentText,
}

impl CommentForm {
    /// Reads every field and says what the form is: a comment, bait, or
    /// the list of what is wrong, so the reader can fix it all at once.
    pub fn read(self) -> Reading {
        if !self.website.trim().is_empty() {
            return Reading::Bait;
        }
        let mut problems = Vec::new();
        let mut keep = |problem| problems.push(problem);
        let name = AuthorName::parse(&self.name).map_err(&mut keep).ok();
        let email = match self.email.trim() {
            "" => Some(None),
            given => ReaderEmail::parse(given).map(Some).map_err(&mut keep).ok(),
        };
        let text = CommentText::parse(&self.comment).map_err(&mut keep).ok();
        let parent = match self.parent.as_deref().map(str::trim) {
            None | Some("") => Some(None),
            Some(id) => id
                .parse()
                .map(|id| Some(CommentId::new(id)))
                .map_err(|_| keep(FieldProblem::ParentInvalid))
                .ok(),
        };
        match (name, email, text, parent) {
            (Some(name), Some(email), Some(text), Some(parent)) => Reading::Comment(FormComment {
                page: self.page,
                parent,
                author: CommentAuthor::Reader { name, email },
                text,
            }),
            _ => Reading::Problems(problems),
        }
    }
}
