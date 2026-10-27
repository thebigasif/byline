//! A running site hears of each new comment and of each change to what a
//! post page shows.

use std::sync::{Arc, Mutex, PoisonError};

use chrono::{TimeZone, Utc};
use khalid_comments::{
    AuthorName, Comment, CommentAuthor, CommentEvents, CommentText, Comments, MemoryCommentStore,
    NewComment, PostFile,
};
use khalid_core::FixedClock;

/// Writes down what it hears, in order.
#[derive(Debug, Default)]
struct Heard(Mutex<Vec<String>>);

impl CommentEvents for Heard {
    fn added(&self, comment: &Comment) {
        let line = format!("added {}", comment.id);
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(line);
    }

    fn shown_changed(&self, post: &PostFile) {
        let line = format!("shown {}", post.as_str());
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(line);
    }
}

#[tokio::test]
async fn each_change_is_told_once_and_a_hidden_change_moves_no_page() {
    let heard = Arc::new(Heard::default());
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap());
    let listener: Arc<dyn CommentEvents> = Arc::<Heard>::clone(&heard);
    let comments = Comments::new(MemoryCommentStore::default(), clock).with_events(listener);
    let new = || NewComment {
        post: PostFile::new("p.md"),
        parent: None,
        author: CommentAuthor::Reader {
            name: AuthorName::parse("Lena").unwrap(),
            email: None,
        },
        text: CommentText::parse("Hi").unwrap(),
    };
    let first = comments.submit(new()).await.unwrap();
    let waiting = comments.submit(new()).await.unwrap();
    let text = CommentText::parse("Thanks").unwrap();
    let staff = AuthorName::parse("Asif").unwrap();
    comments.reply(first.id, staff, text).await.unwrap();
    comments.delete(waiting.id).await.unwrap();
    comments.delete(first.id).await.unwrap();
    let heard = heard.0.lock().unwrap().clone();
    assert_eq!(
        heard,
        ["added 1", "added 2", "added 3", "shown p.md", "shown p.md"]
    );
}
