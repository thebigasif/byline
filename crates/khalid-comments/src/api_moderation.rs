//! The Admin API's moderation, answered by [`Comments`].

use std::sync::Arc;

use khalid_api::comment_moderation::{CommentModeration, ModerationError};
use khalid_api::content_store::BoxFuture;
use khalid_api::resources::{self, AdminComment, CommentReply, CommentWriter};

use crate::{
    AuthorName, Comment, CommentAuthor, CommentId, CommentStatus, CommentStore, CommentText,
    Comments, CommentsError, FieldProblem, ReaderEmail,
};

/// [`Comments`] behind the Admin API's [`CommentModeration`], so the API
/// keeps the same rules as the reader form (ADR 0003).
#[derive(Debug)]
pub struct ApiModeration<S>(Arc<Comments<S>>);

impl<S> ApiModeration<S> {
    /// The Admin API's view of `comments`.
    pub fn new(comments: Arc<Comments<S>>) -> Self {
        Self(comments)
    }
}

/// Which fields of a comment reach whoever reads its API shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Audience {
    /// Staff moderating through the Admin API, who see a reader's email.
    Moderators,
    /// Another server, through a webhook, which never gets the email.
    Integrations,
}

/// `comment` in the API's shape, as `audience` may read it.
pub fn api_shape(comment: &Comment, audience: Audience) -> AdminComment {
    let (kind, email) = match &comment.author {
        CommentAuthor::Reader { email, .. } => (CommentWriter::Reader, email.as_ref()),
        CommentAuthor::Staff { .. } => (CommentWriter::Staff, None),
    };
    let email = email.filter(|_| audience == Audience::Moderators);
    AdminComment {
        id: comment.id.get(),
        post: comment.post.as_str().to_owned(),
        parent: comment.parent.map(CommentId::get),
        author_name: comment.author.name().as_str().to_owned(),
        author_kind: kind,
        author_email: email.map(ReaderEmail::as_str).map(str::to_owned),
        text: comment.text.as_str().to_owned(),
        status: match comment.status {
            CommentStatus::Pending => resources::CommentStatus::Pending,
            CommentStatus::Approved => resources::CommentStatus::Approved,
            CommentStatus::Spam => resources::CommentStatus::Spam,
            CommentStatus::Trash => resources::CommentStatus::Trash,
        },
        created_at: comment.created_at,
    }
}

fn status_of(status: resources::CommentStatus) -> CommentStatus {
    match status {
        resources::CommentStatus::Pending => CommentStatus::Pending,
        resources::CommentStatus::Approved => CommentStatus::Approved,
        resources::CommentStatus::Spam => CommentStatus::Spam,
        resources::CommentStatus::Trash => CommentStatus::Trash,
    }
}

fn moderation_error(error: CommentsError) -> ModerationError {
    match error {
        CommentsError::NotFound { .. } | CommentsError::ParentNotFound { .. } => {
            ModerationError::NotFound
        }
        error => ModerationError::Store(Box::new(error)),
    }
}

fn invalid(field: &'static str, problem: FieldProblem) -> ModerationError {
    let reason = problem.to_string();
    ModerationError::Invalid { field, reason }
}

impl<S: CommentStore + 'static> CommentModeration for ApiModeration<S> {
    fn list(
        &self,
        status: Option<resources::CommentStatus>,
        limit: u32,
    ) -> BoxFuture<'_, Result<Vec<AdminComment>, ModerationError>> {
        Box::pin(async move {
            let listed = self.0.listed(status.map(status_of), limit).await;
            let listed = listed.map_err(moderation_error)?;
            let shape = |comment: &Comment| api_shape(comment, Audience::Moderators);
            Ok(listed.iter().map(shape).collect())
        })
    }

    fn approve(&self, id: i64) -> BoxFuture<'_, Result<AdminComment, ModerationError>> {
        Box::pin(async move {
            let approved = self.0.approve(CommentId::new(id)).await;
            let approved = approved.map_err(moderation_error)?;
            Ok(api_shape(&approved, Audience::Moderators))
        })
    }

    fn reply(
        &self,
        id: i64,
        reply: CommentReply,
    ) -> BoxFuture<'_, Result<AdminComment, ModerationError>> {
        Box::pin(async move {
            let name = AuthorName::parse(&reply.author_name);
            let name = name.map_err(|problem| invalid("author_name", problem))?;
            let text = CommentText::parse(&reply.text);
            let text = text.map_err(|problem| invalid("text", problem))?;
            let answered = self.0.reply(CommentId::new(id), name, text).await;
            let reply = answered.map_err(moderation_error)?;
            Ok(api_shape(&reply, Audience::Moderators))
        })
    }

    fn delete(&self, id: i64) -> BoxFuture<'_, Result<(), ModerationError>> {
        Box::pin(async move {
            let deleted = self.0.delete(CommentId::new(id)).await;
            deleted.map(drop).map_err(moderation_error)
        })
    }
}
