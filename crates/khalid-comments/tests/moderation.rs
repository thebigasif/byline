//! A reader's comment waits for approval; staff approve, reply and delete.

use chrono::{TimeZone, Utc};
use khalid_comments::{
    AuthorName, Comment, CommentAuthor, CommentId, CommentStatus, CommentText, Comments,
    CommentsError, MemoryCommentStore, NewComment, PostFile, Thread,
};
use khalid_core::FixedClock;

#[cfg(test)]
fn comments() -> Comments<MemoryCommentStore> {
    let at = Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap();
    Comments::new(MemoryCommentStore::default(), FixedClock::new(at))
}

#[cfg(test)]
fn post() -> PostFile {
    PostFile::new("content/en/posts/why-links-matter.md")
}

#[cfg(test)]
fn by_reader(parent: Option<CommentId>, text: &str) -> NewComment {
    NewComment {
        post: post(),
        parent,
        author: CommentAuthor::Reader {
            name: AuthorName::parse("Lena").unwrap(),
            email: None,
        },
        text: CommentText::parse(text).unwrap(),
    }
}

#[cfg(test)]
fn staff() -> AuthorName {
    AuthorName::parse("Asif").unwrap()
}

#[cfg(test)]
fn texts(shown: &[Comment]) -> Vec<&str> {
    shown.iter().map(|comment| comment.text.as_str()).collect()
}

#[tokio::test]
async fn a_readers_comment_waits_until_it_is_approved() {
    let comments = comments();
    let sent = comments.submit(by_reader(None, "First!")).await.unwrap();
    assert_eq!(sent.status, CommentStatus::Pending);
    assert_eq!(
        comments.shown_on(&post()).await.unwrap(),
        Vec::<Thread>::new()
    );
    let approved = comments.approve(sent.id).await.unwrap();
    assert_eq!(approved.status, CommentStatus::Approved);
    let shown = comments.shown_on(&post()).await.unwrap();
    assert_eq!(shown.len(), 1);
    assert_eq!(texts(&[shown[0].comment.clone()]), ["First!"]);
}

#[tokio::test]
async fn a_reply_to_a_reply_is_stored_under_the_first_comment() {
    let comments = comments();
    let first = comments.submit(by_reader(None, "First")).await.unwrap();
    comments.approve(first.id).await.unwrap();
    let second = comments
        .submit(by_reader(Some(first.id), "Second"))
        .await
        .unwrap();
    comments.approve(second.id).await.unwrap();
    let third = comments
        .submit(by_reader(Some(second.id), "Third"))
        .await
        .unwrap();
    assert_eq!(third.parent, Some(first.id));
}

#[tokio::test]
async fn a_reader_can_only_answer_a_shown_comment_on_the_same_post() {
    let comments = comments();
    let waiting = comments.submit(by_reader(None, "Waiting")).await.unwrap();
    let refused = comments.submit(by_reader(Some(waiting.id), "Hi")).await;
    assert!(matches!(refused, Err(CommentsError::ParentNotFound { .. })));
    let missing = comments
        .submit(by_reader(Some(CommentId::new(99)), "Hi"))
        .await;
    assert!(matches!(missing, Err(CommentsError::ParentNotFound { .. })));
    comments.approve(waiting.id).await.unwrap();
    let mut elsewhere = by_reader(Some(waiting.id), "Hi");
    elsewhere.post = PostFile::new("content/en/posts/another.md");
    let refused = comments.submit(elsewhere).await;
    assert!(matches!(refused, Err(CommentsError::ParentNotFound { .. })));
}

#[tokio::test]
async fn a_staff_reply_is_shown_at_once_and_approves_what_it_answers() {
    let comments = comments();
    let question = comments.submit(by_reader(None, "Why?")).await.unwrap();
    let text = CommentText::parse("Because.").unwrap();
    let reply = comments.reply(question.id, staff(), text).await.unwrap();
    assert_eq!(reply.status, CommentStatus::Approved);
    assert_eq!(reply.parent, Some(question.id));
    assert!(matches!(reply.author, CommentAuthor::Staff { .. }));
    let shown = comments.shown_on(&post()).await.unwrap();
    assert_eq!(texts(&[shown[0].comment.clone()]), ["Why?"]);
    assert_eq!(texts(&shown[0].replies), ["Because."]);
}

#[tokio::test]
async fn deleting_a_comment_takes_its_replies_with_it() {
    let comments = comments();
    let first = comments.submit(by_reader(None, "First")).await.unwrap();
    let text = CommentText::parse("Reply").unwrap();
    comments.reply(first.id, staff(), text).await.unwrap();
    let gone = comments.delete(first.id).await.unwrap();
    assert_eq!(gone.id, first.id);
    assert_eq!(
        comments.shown_on(&post()).await.unwrap(),
        Vec::<Thread>::new()
    );
    assert_eq!(
        comments.listed(None, 10).await.unwrap(),
        Vec::<Comment>::new()
    );
}

#[tokio::test]
async fn moderating_a_missing_comment_says_so() {
    let comments = comments();
    let missing = CommentId::new(7);
    let not_found = |result| matches!(result, Err(CommentsError::NotFound { id }) if id == missing);
    assert!(not_found(comments.approve(missing).await));
    assert!(not_found(comments.delete(missing).await));
    let text = CommentText::parse("Hello").unwrap();
    assert!(not_found(comments.reply(missing, staff(), text).await));
}

#[tokio::test]
async fn the_queue_lists_newest_first_and_filters_by_status() {
    let comments = comments();
    let one = comments.submit(by_reader(None, "One")).await.unwrap();
    comments.submit(by_reader(None, "Two")).await.unwrap();
    comments.submit(by_reader(None, "Three")).await.unwrap();
    comments.approve(one.id).await.unwrap();
    let pending = comments
        .listed(Some(CommentStatus::Pending), 10)
        .await
        .unwrap();
    assert_eq!(texts(&pending), ["Three", "Two"]);
    assert_eq!(
        texts(&comments.listed(None, 2).await.unwrap()),
        ["Three", "Two"]
    );
}

#[tokio::test]
async fn every_post_with_an_approved_comment_is_shown() {
    let comments = comments();
    let here = comments.submit(by_reader(None, "Here")).await.unwrap();
    comments.approve(here.id).await.unwrap();
    let mut elsewhere = by_reader(None, "Waiting");
    elsewhere.post = PostFile::new("content/en/posts/another.md");
    comments.submit(elsewhere).await.unwrap();
    let shown = comments.shown().await.unwrap();
    let posts: Vec<&PostFile> = shown.keys().collect();
    assert_eq!(posts, [&post()]);
}
