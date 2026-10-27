//! The in-memory store keeps the contract `khalid-db` keeps on SQLite.

use chrono::{TimeZone, Utc};
use khalid_comments::{
    AuthorName, CommentAuthor, CommentId, CommentStatus, CommentStore, CommentText,
    MemoryCommentStore, NewComment, PostFile,
};

#[cfg(test)]
fn new(post: &str, parent: Option<i64>) -> NewComment {
    NewComment {
        post: PostFile::new(post),
        parent: parent.map(CommentId::new),
        author: CommentAuthor::Staff {
            name: AuthorName::parse("Asif").unwrap(),
        },
        text: CommentText::parse("Thanks.").unwrap(),
    }
}

#[tokio::test]
async fn rows_are_numbered_changed_found_and_removed_with_their_replies() {
    use CommentStatus::{Approved, Pending};
    let store = MemoryCommentStore::default();
    let at = Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap();
    let (a, b) = ("content/en/posts/a.md", "content/en/posts/b.md");
    let first = store.insert(new(a, None), Pending, at).await.unwrap();
    let reply = store.insert(new(a, Some(1)), Approved, at).await.unwrap();
    let other = store.insert(new(b, None), Approved, at).await.unwrap();
    let ids = |rows: Vec<khalid_comments::Comment>| -> Vec<i64> {
        rows.iter().map(|row| row.id.get()).collect()
    };
    assert_eq!(
        [first.id, reply.id, other.id].map(CommentId::get),
        [1, 2, 3]
    );
    assert_eq!(store.get(first.id).await.unwrap(), Some(first.clone()));
    let on_a = store.with_status(Approved, Some(&PostFile::new(a))).await;
    assert_eq!(ids(on_a.unwrap()), [2]);
    assert_eq!(
        ids(store.with_status(Approved, None).await.unwrap()),
        [2, 3]
    );
    assert_eq!(ids(store.newest(None, 2).await.unwrap()), [3, 2]);
    assert_eq!(ids(store.newest(Some(Pending), 9).await.unwrap()), [1]);
    let approved = store.set_status(first.id, Approved).await.unwrap();
    assert_eq!(approved.map(|row| row.status), Some(Approved));
    assert_eq!(
        store.delete(first.id).await.unwrap(),
        Some(approved_first(first))
    );
    assert_eq!(ids(store.newest(None, 9).await.unwrap()), [3]);
    let missing = CommentId::new(9);
    assert_eq!(store.set_status(missing, Approved).await.unwrap(), None);
    assert_eq!(store.delete(missing).await.unwrap(), None);
}

#[cfg(test)]
fn approved_first(mut first: khalid_comments::Comment) -> khalid_comments::Comment {
    first.status = CommentStatus::Approved;
    first
}
