//! A post's schedule: when it goes live and when it comes down, kept by a
//! `ScheduleStore`, with the due ones handed out oldest first (CTO answers
//! 2026-10-27 section 4).

use chrono::{DateTime, TimeZone, Utc};
use khalid_api::resources::{AdminPost, PostStatus};
use khalid_api::schedule_store::{Due, MemoryScheduleStore, Schedule, ScheduleStore};
use khalid_core::Slug;

#[cfg(test)]
fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 3, hour, 0, 0).unwrap()
}

#[test]
fn a_schedule_names_its_times() {
    let schedule = Schedule::default()
        .publishing_at(at(9))
        .unpublishing_at(at(18));
    assert_eq!(schedule.publish_at(), Some(at(9)));
    assert_eq!(schedule.unpublish_at(), Some(at(18)));
    assert!(!schedule.is_empty());
    assert!(Schedule::default().is_empty());
}

#[tokio::test]
async fn a_post_with_no_schedule_has_an_empty_one() {
    let store = MemoryScheduleStore::default();
    assert_eq!(
        store.get("en.choosing-anchor-text").await.unwrap(),
        Schedule::default()
    );
}

#[tokio::test]
async fn what_is_set_comes_back_and_an_empty_schedule_clears_it() {
    let store = MemoryScheduleStore::default();
    let schedule = Schedule::default().publishing_at(at(9));
    store
        .set("en.choosing-anchor-text", schedule)
        .await
        .unwrap();
    assert_eq!(
        store.get("en.choosing-anchor-text").await.unwrap(),
        schedule
    );
    store
        .set("en.choosing-anchor-text", Schedule::default())
        .await
        .unwrap();
    assert_eq!(store.due(at(23)).await.unwrap(), []);
}

#[tokio::test]
async fn due_times_come_oldest_first_and_later_ones_wait() {
    let store = MemoryScheduleStore::default();
    let first = Schedule::default()
        .publishing_at(at(9))
        .unpublishing_at(at(18));
    store.set("en.choosing-anchor-text", first).await.unwrap();
    store
        .set(
            "en.what-a-sitemap-does",
            Schedule::default().publishing_at(at(8)),
        )
        .await
        .unwrap();
    let due = store.due(at(10)).await.unwrap();
    assert_eq!(
        due,
        [
            ("en.what-a-sitemap-does".to_owned(), Due::Publish),
            ("en.choosing-anchor-text".to_owned(), Due::Publish),
        ]
    );
    let later = store.due(at(19)).await.unwrap();
    assert!(later.contains(&("en.choosing-anchor-text".to_owned(), Due::Unpublish)));
}

#[test]
fn a_draft_with_a_publish_time_shows_as_scheduled_and_a_live_post_stays_live() {
    let draft = AdminPost::new(
        "en.choosing-anchor-text",
        Slug::try_new("choosing-anchor-text").unwrap(),
        "Choosing anchor text",
        "Words.",
        at(7),
    );
    let schedule = Schedule::default()
        .publishing_at(at(9))
        .unpublishing_at(at(18));
    let scheduled = draft.clone().with_schedule(&schedule);
    assert_eq!(scheduled.status(), PostStatus::Scheduled);
    assert_eq!(scheduled.published_at(), Some(at(9)));
    assert_eq!(scheduled.unpublish_at(), Some(at(18)));

    let live = draft.published(at(8)).with_schedule(&schedule);
    assert_eq!(live.status(), PostStatus::Published);
    assert_eq!(live.published_at(), Some(at(8)));
    assert_eq!(live.unpublish_at(), Some(at(18)));
}
