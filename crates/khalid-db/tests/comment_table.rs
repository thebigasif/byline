//! Comments on SQLite keep the store contract, and each site sees only its own.

use std::path::PathBuf;

use chrono::{DateTime, TimeDelta, TimeZone, Utc};
use khalid_comments::{
    AuthorName, Comment, CommentAuthor, CommentId, CommentStatus, CommentStore, CommentText,
    NewComment, PostFile, ReaderEmail,
};
use khalid_core::Slug;
use khalid_db::{CommentTable, Database};

#[cfg(test)]
fn at(minutes: i64) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap() + TimeDelta::minutes(minutes)
}

/// A migrated database with the comments of the sites `demo` and `other`.
#[cfg(test)]
async fn opened() -> (tempfile::TempDir, CommentTable, CommentTable) {
    let dir = tempfile::tempdir().unwrap();
    let mut path = PathBuf::from(dir.path());
    path.push("site.db");
    let db = Database::open(&path).await.unwrap();
    db.migrate().await.unwrap();
    let demo = db.site(&Slug::try_new("demo").unwrap()).await.unwrap();
    let other = db.site(&Slug::try_new("other").unwrap()).await.unwrap();
    let tables = (
        CommentTable::new(db.clone(), demo),
        CommentTable::new(db, other),
    );
    (dir, tables.0, tables.1)
}

#[cfg(test)]
fn reader(post: &str, parent: Option<CommentId>, email: Option<&str>) -> NewComment {
    NewComment {
        post: PostFile::new(post),
        parent,
        author: CommentAuthor::Reader {
            name: AuthorName::parse("Lena Vogt").unwrap(),
            email: email.map(|email| ReaderEmail::parse(email).unwrap()),
        },
        text: CommentText::parse("Great post.\n\nThanks!").unwrap(),
    }
}

#[cfg(test)]
fn ids(rows: &[Comment]) -> Vec<i64> {
    rows.iter().map(|row| row.id.get()).collect()
}

#[tokio::test]
async fn a_comment_comes_back_as_it_went_in() {
    use CommentStatus::{Approved, Pending};
    let (_dir, comments, _) = opened().await;
    let post = "content/en/posts/why-links-matter.md";
    let first = comments
        .insert(reader(post, None, Some("lena@example.com")), Pending, at(0))
        .await
        .unwrap();
    let staff = NewComment {
        author: CommentAuthor::Staff {
            name: AuthorName::parse("Asif").unwrap(),
        },
        ..reader(post, Some(first.id), None)
    };
    let reply = comments.insert(staff, Approved, at(1)).await.unwrap();
    assert_eq!(comments.get(first.id).await.unwrap(), Some(first.clone()));
    assert_eq!(comments.get(reply.id).await.unwrap(), Some(reply.clone()));
    assert_eq!(reply.parent, Some(first.id));
    assert_eq!(reply.created_at, at(1));
    let no_email = comments
        .insert(reader(post, None, None), Pending, at(2))
        .await
        .unwrap();
    assert_eq!(comments.get(no_email.id).await.unwrap(), Some(no_email));
}

#[tokio::test]
async fn statuses_lists_and_deletes_follow_the_contract() {
    use CommentStatus::{Approved, Pending};
    let (_dir, comments, _) = opened().await;
    let (a, b) = ("content/en/posts/a.md", "content/en/posts/b.md");
    let first = comments.insert(reader(a, None, None), Pending, at(0)).await;
    let first = first.unwrap();
    let reply = reader(a, Some(first.id), None);
    let reply = comments.insert(reply, Approved, at(1)).await.unwrap();
    let other = comments
        .insert(reader(b, None, None), Approved, at(2))
        .await;
    let other = other.unwrap();
    let on_a = comments
        .with_status(Approved, Some(&PostFile::new(a)))
        .await;
    assert_eq!(ids(&on_a.unwrap()), [reply.id.get()]);
    let everywhere = comments.with_status(Approved, None).await.unwrap();
    assert_eq!(ids(&everywhere), [reply.id.get(), other.id.get()]);
    let newest = comments.newest(None, 2).await.unwrap();
    assert_eq!(ids(&newest), [other.id.get(), reply.id.get()]);
    let waiting = comments.newest(Some(Pending), 9).await.unwrap();
    assert_eq!(ids(&waiting), [first.id.get()]);
    let approved = comments.set_status(first.id, Approved).await.unwrap();
    assert_eq!(approved.map(|row| row.status), Some(Approved));
    let deleted = comments.delete(first.id).await.unwrap();
    assert_eq!(deleted.map(|row| row.id), Some(first.id));
    assert_eq!(
        comments.get(reply.id).await.unwrap(),
        None,
        "a reply goes with its comment"
    );
    let missing = CommentId::new(99);
    assert_eq!(comments.set_status(missing, Approved).await.unwrap(), None);
    assert_eq!(comments.delete(missing).await.unwrap(), None);
}

#[tokio::test]
async fn a_site_never_sees_or_changes_another_sites_comments() {
    use CommentStatus::{Approved, Pending};
    let (_dir, demo, other) = opened().await;
    let post = "content/en/posts/a.md";
    let mine = demo.insert(reader(post, None, None), Pending, at(0)).await;
    let mine = mine.unwrap();
    assert_eq!(other.get(mine.id).await.unwrap(), None);
    assert_eq!(other.set_status(mine.id, Approved).await.unwrap(), None);
    assert_eq!(other.delete(mine.id).await.unwrap(), None);
    assert_eq!(other.newest(None, 9).await.unwrap(), Vec::<Comment>::new());
    assert_eq!(demo.get(mine.id).await.unwrap(), Some(mine));
}
