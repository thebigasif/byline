//! Work that follows a render hears each time the served pages are
//! replaced, and nothing in between.

use std::time::Duration;

use khalid_server::{LivePages, MediaType, Page, PagePath, Pages};

/// Long enough that a replacement already made has long been heard.
const SETTLE: Duration = Duration::from_millis(200);

#[cfg(test)]
fn home() -> Pages {
    let path = PagePath::try_new("/blog/").unwrap();
    let page = Page::new(path, MediaType::Html, "<h1>Generated Field Notes</h1>");
    Pages::first(vec![page]).unwrap()
}

#[tokio::test]
async fn nothing_is_heard_until_the_pages_are_replaced() {
    let live = LivePages::new(home());
    let mut replacements = live.replacements();
    let heard = tokio::time::timeout(SETTLE, replacements.next()).await;
    assert!(heard.is_err(), "the pages were never replaced");
}

#[tokio::test]
async fn several_replacements_in_a_row_are_heard_once() {
    let live = LivePages::new(home());
    let mut replacements = live.replacements();
    live.show_saved(Vec::new());
    live.replace_if_current(&live.current(), home());
    tokio::time::timeout(SETTLE, replacements.next())
        .await
        .expect("a replacement is heard");
    let again = tokio::time::timeout(SETTLE, replacements.next()).await;
    assert!(again.is_err(), "two replacements in a row count as one");
    live.show_saved(Vec::new());
    tokio::time::timeout(SETTLE, replacements.next())
        .await
        .expect("a later replacement is heard too");
}
