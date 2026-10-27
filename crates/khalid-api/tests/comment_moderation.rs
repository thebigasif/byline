//! A comment's API shape, and a site that keeps no comments.

use chrono::{TimeZone, Utc};
use khalid_api::comment_moderation::{CommentModeration, ModerationError, NoComments};
use khalid_api::resources::{AdminComment, CommentReply, CommentStatus, CommentWriter};
use serde_json::json;

#[test]
fn a_comment_is_sent_as_flat_json_and_an_absent_email_is_left_out() {
    let mut comment = AdminComment {
        id: 3,
        post: "content/en/posts/why-links-matter.md".to_owned(),
        parent: Some(1),
        author_name: "Asif".to_owned(),
        author_kind: CommentWriter::Staff,
        author_email: None,
        text: "Thanks, Lena.".to_owned(),
        status: CommentStatus::Approved,
        created_at: Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap(),
    };
    assert_eq!(
        serde_json::to_value(&comment).unwrap(),
        json!({
            "id": 3,
            "post": "content/en/posts/why-links-matter.md",
            "parent": 1,
            "author_name": "Asif",
            "author_kind": "staff",
            "text": "Thanks, Lena.",
            "status": "approved",
            "created_at": "2026-10-27T09:00:00Z"
        })
    );
    comment.author_email = Some("lena@example.com".to_owned());
    let sent = serde_json::to_value(&comment).unwrap();
    assert_eq!(sent["author_email"], "lena@example.com");
}

#[test]
fn a_status_reads_from_its_lowercase_name() {
    let read: CommentStatus = serde_json::from_value(json!("pending")).unwrap();
    assert_eq!(read, CommentStatus::Pending);
    assert!(serde_json::from_value::<CommentStatus>(json!("Pending")).is_err());
}

#[tokio::test]
async fn a_site_without_comments_lists_none_and_finds_none() {
    let none = NoComments;
    let listed = none.list(Some(CommentStatus::Pending), 50).await.unwrap();
    assert_eq!(listed, Vec::<AdminComment>::new());
    let not_found =
        |result: Result<_, ModerationError>| matches!(result, Err(ModerationError::NotFound));
    assert!(not_found(none.approve(1).await.map(drop)));
    assert!(not_found(none.delete(1).await));
    let reply = CommentReply {
        author_name: "Asif".to_owned(),
        text: "Hello".to_owned(),
    };
    assert!(not_found(none.reply(1, reply).await.map(drop)));
}
