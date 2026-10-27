//! Approved comments as a post shows them: one level of replies.

use std::collections::BTreeMap;

use crate::{Comment, CommentId, CommentStatus};

/// A comment with the replies under it, oldest first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Thread {
    /// The comment that started the thread.
    pub comment: Comment,
    /// Its approved replies.
    pub replies: Vec<Comment>,
}

/// The approved ones among `comments`, all on one post, as threads one
/// level deep, oldest first; comments sent in the same second keep the
/// order they were stored in.
///
/// A reply to a reply joins the thread of the comment that started it. A
/// reply whose thread is not approved is not shown, since it would answer
/// something the reader cannot see.
pub fn threads(comments: Vec<Comment>) -> Vec<Thread> {
    let mut shown: Vec<Comment> = comments
        .into_iter()
        .filter(|comment| comment.status == CommentStatus::Approved)
        .collect();
    shown.sort_by_key(|comment| (comment.created_at, comment.id));
    let parents: BTreeMap<CommentId, Option<CommentId>> = shown
        .iter()
        .map(|comment| (comment.id, comment.parent))
        .collect();
    let mut started: Vec<Thread> = Vec::new();
    let mut replies = Vec::new();
    for comment in shown {
        match comment.parent {
            None => started.push(Thread {
                comment,
                replies: Vec::new(),
            }),
            Some(parent) => replies.push((first_of(&parents, parent), comment)),
        }
    }
    for (first, reply) in replies {
        let thread = started
            .iter_mut()
            .find(|thread| Some(thread.comment.id) == first);
        if let Some(thread) = thread {
            thread.replies.push(reply);
        }
    }
    started
}

/// The comment that starts the thread `id` is in, among the shown
/// comments and their `parents`. `None` when the chain leaves them.
fn first_of(parents: &BTreeMap<CommentId, Option<CommentId>>, id: CommentId) -> Option<CommentId> {
    let mut at = id;
    // A chain cannot be longer than the comments in it; stopping there
    // also ends a loop that bad data could make.
    for _ in 0..=parents.len() {
        match parents.get(&at)? {
            None => return Some(at),
            Some(parent) => at = *parent,
        }
    }
    None
}
