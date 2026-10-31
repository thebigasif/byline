//! The memory store of comments on drafts keeps each post's comments
//! apart, dates them, and resolves only a thread's first comment, as
//! every store must (KH-EDT-4).

use chrono::{TimeZone, Utc};
use khalid_api::draft_comments::{
    DraftCommentStore, DraftCommentText, MemoryDraftComments, NewDraftComment, ThreadState,
};
use khalid_core::FixedClock;

#[cfg(test)]
fn text(words: &str) -> DraftCommentText {
    DraftCommentText::try_from(words.to_owned()).unwrap()
}

#[tokio::test]
async fn comments_are_kept_per_post_dated_and_resolved_by_their_thread() {
    let at = Utc.with_ymd_and_hms(2026, 10, 20, 8, 30, 0).unwrap();
    let store = MemoryDraftComments::with_clock(FixedClock::new(at));
    let mut first = NewDraftComment::new("en.notes", "editor", text("Name the tool?"));
    first.block = Some(2);
    let first = store.add(first).await.unwrap();
    let mut reply = NewDraftComment::new("en.notes", "writer", text("Done."));
    (reply.parent, reply.block) = (Some(first.id), first.block);
    let reply = store.add(reply).await.unwrap();
    store
        .add(NewDraftComment::new(
            "en.other",
            "editor",
            text("Elsewhere."),
        ))
        .await
        .unwrap();

    let listed = store.list("en.notes").await.unwrap();
    assert_eq!(listed, [first.clone(), reply.clone()]);
    assert_eq!(listed[0].created_at, at);
    assert_eq!(listed[1].block, Some(2));

    let resolved = store.resolve("en.notes", first.id, ThreadState::Resolved);
    assert!(resolved.await.unwrap());
    assert!(store.list("en.notes").await.unwrap()[0].resolved);
    let reply_resolved = store.resolve("en.notes", reply.id, ThreadState::Resolved);
    assert!(!reply_resolved.await.unwrap(), "a reply begins no thread");
    let elsewhere = store.resolve("en.other", first.id, ThreadState::Resolved);
    assert!(!elsewhere.await.unwrap(), "a thread on another post");
}

#[test]
fn text_must_say_something_and_stay_short() {
    assert!(DraftCommentText::try_from("  ".to_owned()).is_err());
    assert!(DraftCommentText::try_from("a".repeat(5_001)).is_err());
    assert!(DraftCommentText::try_from("a".repeat(5_000)).is_ok());
}
