//! Who may move a draft between review states (KH-EDT-4, plan section
//! 4.6): a writer submits, an editor approves.

use khalid_core::UserMessage;

use crate::problem::{Problem, ProblemType};
use crate::resources::{AdminPost, PostEdit, ReviewState};

/// Checks an edit to `post` by someone without `posts:publish`: a writer,
/// a contributor or a drafts-only key. They move a draft between `draft`
/// and `in_review` freely, but only an editor or above marks it `ready`.
/// An approved post stays approved only while nobody without that right
/// changes it, so their edit must send it back to `draft` or `in_review`.
///
/// # Errors
///
/// Returns a 403 [`Problem`] naming `review_state` when the post would be
/// `ready` after the edit.
pub(crate) fn check(post: &AdminPost, edit: &PostEdit) -> Result<(), Problem> {
    if edit.review_state_for(post) != ReviewState::Ready {
        return Ok(());
    }
    let message = if post.settings().review_state == ReviewState::Ready {
        UserMessage::new(
            "An editor has approved this post, so only an editor may change it as it is.",
            "A change after approval needs the editor to look again.",
            "Send review_state as in_review with your change, and an editor approves it again.",
        )
    } else {
        UserMessage::new(
            "Only an editor, an admin or the owner may mark a post ready.",
            "Ready means an editor has read the post and approved it.",
            "Send review_state as in_review, and an editor approves it.",
        )
    };
    Err(Problem::new(ProblemType::Forbidden, &message).with_rule("review_state"))
}
