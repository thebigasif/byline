//! The Admin API moderates through the same rules as the reader form.

use std::sync::Arc;

use chrono::{TimeZone, Utc};
use khalid_api::comment_moderation::{CommentModeration, ModerationError};
use khalid_api::resources::{self, CommentReply, CommentWriter};
use khalid_comments::{
    ApiModeration, AuthorName, CommentAuthor, CommentText, Comments, MemoryCommentStore,
    NewComment, PostFile, ReaderEmail,
};
use khalid_core::FixedClock;

#[cfg(test)]
async fn one_waiting() -> ApiModeration<MemoryCommentStore> {
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap());
    let comments = Comments::new(MemoryCommentStore::default(), clock);
    let new = NewComment {
        post: PostFile::new("content/en/posts/why-links-matter.md"),
        parent: None,
        author: CommentAuthor::Reader {
            name: AuthorName::parse("Lena").unwrap(),
            email: Some(ReaderEmail::parse("lena@example.com").unwrap()),
        },
        text: CommentText::parse("Great post.").unwrap(),
    };
    comments.submit(new).await.unwrap();
    ApiModeration::new(Arc::new(comments))
}

#[cfg(test)]
fn reply(name: &str, text: &str) -> CommentReply {
    CommentReply {
        author_name: name.to_owned(),
        text: text.to_owned(),
    }
}

#[tokio::test]
async fn moderators_see_the_queue_with_the_readers_email() {
    let api = one_waiting().await;
    let queue = api.list(Some(resources::CommentStatus::Pending), 10).await;
    let queue = queue.unwrap();
    assert_eq!(queue.len(), 1);
    assert_eq!(queue[0].author_email.as_deref(), Some("lena@example.com"));
    assert_eq!(queue[0].author_kind, CommentWriter::Reader);
    let approved = api.list(Some(resources::CommentStatus::Approved), 10).await;
    assert_eq!(approved.unwrap().len(), 0);
}

#[tokio::test]
async fn a_reply_approves_and_answers_and_delete_takes_both() {
    let api = one_waiting().await;
    let answer = api.reply(1, reply("Asif", "Thanks, Lena.")).await.unwrap();
    assert_eq!(answer.parent, Some(1));
    assert_eq!(answer.author_kind, CommentWriter::Staff);
    assert_eq!(answer.status, resources::CommentStatus::Approved);
    let shown = api.list(Some(resources::CommentStatus::Approved), 10).await;
    assert_eq!(shown.unwrap().len(), 2);
    api.delete(1).await.unwrap();
    assert_eq!(api.list(None, 10).await.unwrap().len(), 0);
}

#[tokio::test]
async fn a_reply_names_the_field_it_cannot_keep_and_a_missing_comment_is_not_found() {
    let api = one_waiting().await;
    let field = |result: Result<_, ModerationError>| match result {
        Err(ModerationError::Invalid { field, .. }) => field,
        other => panic!("expected an invalid field, got {other:?}"),
    };
    assert_eq!(field(api.reply(1, reply(" ", "Hi")).await), "author_name");
    assert_eq!(field(api.reply(1, reply("Asif", "")).await), "text");
    let missing = api.approve(9).await;
    assert!(matches!(missing, Err(ModerationError::NotFound)));
    assert!(matches!(
        api.delete(9).await,
        Err(ModerationError::NotFound)
    ));
}
