//! Comments on drafts on SQLite: threads read back as written, resolve by
//! their first comment, and each site sees only its own (KH-EDT-4).

use std::path::PathBuf;

use chrono::{TimeZone, Utc};
use khalid_api::draft_comments::{
    DraftCommentStore, DraftCommentText, NewDraftComment, ThreadState,
};
use khalid_core::{FixedClock, Slug};
use khalid_db::{Database, DraftCommentTable};

/// A migrated database with the sites `demo` and `other`, and the comment
/// table of each.
#[cfg(test)]
async fn opened() -> (tempfile::TempDir, DraftCommentTable, DraftCommentTable) {
    let dir = tempfile::tempdir().unwrap();
    let mut path = PathBuf::from(dir.path());
    path.push("site.db");
    let db = Database::open(&path).await.unwrap();
    db.migrate().await.unwrap();
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap());
    let demo = db.site(&Slug::try_new("demo").unwrap()).await.unwrap();
    let other = db.site(&Slug::try_new("other").unwrap()).await.unwrap();
    let demo = DraftCommentTable::new(db.clone(), demo, clock);
    (dir, demo, DraftCommentTable::new(db, other, clock))
}

#[cfg(test)]
fn comment(author: &str, words: &str) -> NewDraftComment {
    let text = DraftCommentText::try_from(words.to_owned()).unwrap();
    NewDraftComment::new("en.notes", author, text)
}

#[tokio::test]
async fn a_thread_reads_back_as_written_and_resolves_by_its_first_comment() {
    let (_dir, demo, other) = opened().await;
    let mut first = comment("3333444455556666", "Name the tool here?");
    first.block = Some(2);
    let first = demo.add(first).await.unwrap();
    let mut reply = comment("2222333344445555", "Done, it is Ahrefs.");
    (reply.parent, reply.block) = (Some(first.id), first.block);
    let reply = demo.add(reply).await.unwrap();
    assert_eq!(
        first.created_at,
        Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap()
    );
    assert_eq!(
        demo.list("en.notes").await.unwrap(),
        [first.clone(), reply.clone()]
    );
    assert!(
        other.list("en.notes").await.unwrap().is_empty(),
        "another site's"
    );

    assert!(
        demo.resolve("en.notes", first.id, ThreadState::Resolved)
            .await
            .unwrap()
    );
    assert!(demo.list("en.notes").await.unwrap()[0].resolved);
    assert!(
        !demo
            .resolve("en.notes", reply.id, ThreadState::Resolved)
            .await
            .unwrap()
    );
    assert!(
        !other
            .resolve("en.notes", first.id, ThreadState::Resolved)
            .await
            .unwrap()
    );
    assert!(
        demo.resolve("en.notes", first.id, ThreadState::Open)
            .await
            .unwrap()
    );
    assert!(!demo.list("en.notes").await.unwrap()[0].resolved);
}
