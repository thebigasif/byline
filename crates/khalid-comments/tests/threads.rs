//! Approved comments are shown one level deep, oldest first.

use chrono::{DateTime, TimeZone, Utc};
use khalid_comments::{
    AuthorName, Comment, CommentAuthor, CommentId, CommentStatus, CommentText, PostFile, threads,
};

#[cfg(test)]
fn at(minute: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 27, 9, minute, 0).unwrap()
}

#[cfg(test)]
fn comment(id: i64, parent: Option<i64>, minute: u32, status: CommentStatus) -> Comment {
    Comment {
        id: CommentId::new(id),
        post: PostFile::new("content/en/posts/why-links-matter.md"),
        parent: parent.map(CommentId::new),
        author: CommentAuthor::Reader {
            name: AuthorName::parse("Lena").unwrap(),
            email: None,
        },
        text: CommentText::parse("Thanks.").unwrap(),
        status,
        created_at: at(minute),
    }
}

#[cfg(test)]
fn ids(shown: &[khalid_comments::Thread]) -> Vec<(i64, Vec<i64>)> {
    shown
        .iter()
        .map(|thread| {
            let replies = thread.replies.iter().map(|reply| reply.id.get()).collect();
            (thread.comment.id.get(), replies)
        })
        .collect()
}

#[test]
fn replies_sit_under_their_comment_oldest_first() {
    use CommentStatus::Approved;
    let shown = threads(vec![
        comment(4, Some(1), 9, Approved),
        comment(2, None, 5, Approved),
        comment(1, None, 1, Approved),
        comment(3, Some(1), 7, Approved),
    ]);
    assert_eq!(ids(&shown), [(1, vec![3, 4]), (2, vec![])]);
}

#[test]
fn a_reply_to_a_reply_joins_the_first_comment() {
    use CommentStatus::Approved;
    let shown = threads(vec![
        comment(1, None, 1, Approved),
        comment(2, Some(1), 2, Approved),
        comment(3, Some(2), 3, Approved),
    ]);
    assert_eq!(ids(&shown), [(1, vec![2, 3])]);
}

#[test]
fn only_approved_comments_show_and_a_reply_under_a_hidden_one_hides() {
    use CommentStatus::{Approved, Pending, Spam, Trash};
    let shown = threads(vec![
        comment(1, None, 1, Approved),
        comment(2, None, 2, Pending),
        comment(3, Some(2), 3, Approved),
        comment(4, Some(1), 4, Spam),
        comment(5, Some(1), 5, Trash),
        comment(6, Some(1), 6, Approved),
    ]);
    assert_eq!(ids(&shown), [(1, vec![6])]);
}

#[test]
fn comments_at_the_same_moment_keep_the_order_they_were_made() {
    use CommentStatus::Approved;
    let shown = threads(vec![
        comment(9, None, 1, Approved),
        comment(8, None, 1, Approved),
    ]);
    assert_eq!(ids(&shown), [(8, vec![]), (9, vec![])]);
}
