//! The scheduler's half of the publish door: `run_due` publishes or takes
//! down each post whose time has come, through the same edit as the API,
//! once, even when it runs again after a restart (CTO answers 2026-10-27
//! section 4). The clock is fixed, so each run sees the time it is given.

use std::sync::{Arc, Mutex};

use chrono::{DateTime, TimeZone, Utc};
use khalid_api::content_store::{BoxFuture, Listed};
use khalid_api::post_store::{
    CreatePostError, DeletePostError, MemoryPostStore, PostStore, PostStoreError, UpdatePostError,
};
use khalid_api::publish_door::{Announce, PublishDoor, Ran};
use khalid_api::query::ListQuery;
use khalid_api::resources::{AdminPost, PostDraft, PostEdit, PostRow, PostStatus};
use khalid_api::schedule_store::{MemoryScheduleStore, Schedule, ScheduleStore};
use khalid_core::{FixedClock, Slug};
use khalid_lint::Report;

/// The draft's identity.
const ID: &str = "en.choosing-anchor-text";

#[cfg(test)]
fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 3, hour, 0, 0).unwrap()
}

/// Writes down each event and the post it was about.
#[derive(Debug, Default)]
struct Recorder(Mutex<Vec<String>>);

#[cfg(test)]
impl Announce for Recorder {
    fn published(&self, post: &AdminPost) {
        self.0.lock().unwrap().push(format!("up {}", post.id()));
    }

    fn unpublished(&self, post: &AdminPost) {
        self.0.lock().unwrap().push(format!("down {}", post.id()));
    }

    fn blocked(&self, post: &AdminPost, _report: &Report) {
        self.0
            .lock()
            .unwrap()
            .push(format!("blocked {}", post.id()));
    }
}

/// The stores a site keeps across restarts, and who was told what.
#[cfg(test)]
struct Site {
    posts: Arc<dyn PostStore>,
    schedules: Arc<MemoryScheduleStore>,
    told: Arc<Recorder>,
}

#[cfg(test)]
impl Site {
    fn new(posts: impl PostStore + 'static) -> Self {
        Self {
            posts: Arc::new(posts),
            schedules: Arc::new(MemoryScheduleStore::default()),
            told: Arc::new(Recorder::default()),
        }
    }

    /// A door over the site as `khalid serve` builds one at start, with
    /// the clock at `hour`.
    fn door(&self, hour: u32) -> PublishDoor {
        let told: Arc<dyn Announce> = Arc::<Recorder>::clone(&self.told);
        let schedules: Arc<dyn ScheduleStore> = Arc::<MemoryScheduleStore>::clone(&self.schedules);
        PublishDoor::new(Arc::clone(&self.posts), told)
            .with_schedules(schedules, FixedClock::new(at(hour)))
    }

    fn told(&self) -> Vec<String> {
        self.told.0.lock().unwrap().clone()
    }
}

#[cfg(test)]
fn draft() -> MemoryPostStore {
    let post = AdminPost::new(
        ID,
        Slug::try_new("choosing-anchor-text").unwrap(),
        "Choosing anchor text",
        "Anchor text tells readers where a link goes.",
        at(7),
    );
    MemoryPostStore::new(vec![post]).with_clock(FixedClock::new(at(8)))
}

#[tokio::test]
async fn a_post_goes_live_at_its_time_and_comes_down_at_its_other_once_each() {
    let site = Site::new(draft());
    let schedule = Schedule::default()
        .publishing_at(at(10))
        .unpublishing_at(at(18));
    site.schedules.set(ID, schedule).await.unwrap();

    assert_eq!(site.door(9).run_due().await.unwrap(), []);
    assert_eq!(
        site.door(10).run_due().await.unwrap(),
        [Ran::Published(ID.to_owned())]
    );
    let live = site.door(11).get(ID).await.unwrap().unwrap();
    assert_eq!(live.status(), PostStatus::Published);
    assert_eq!(live.unpublish_at(), Some(at(18)));

    // A restart runs the scheduler again; nothing is due twice.
    assert_eq!(site.door(12).run_due().await.unwrap(), []);
    assert_eq!(
        site.door(18).run_due().await.unwrap(),
        [Ran::Unpublished(ID.to_owned())]
    );
    assert_eq!(site.door(19).run_due().await.unwrap(), []);
    assert_eq!(site.told(), [format!("up {ID}"), format!("down {ID}")]);
    assert_eq!(site.schedules.get(ID).await.unwrap(), Schedule::default());
}

#[tokio::test]
async fn a_post_already_live_or_gone_when_its_time_comes_is_settled_quietly() {
    // As after a crash between publishing and clearing the time.
    let site = Site::new(draft());
    let up = PostEdit::status_only(PostStatus::Published, at(7));
    site.posts.update(ID, up, "asif").await.unwrap();
    let schedule = Schedule::default().publishing_at(at(10));
    site.schedules.set(ID, schedule).await.unwrap();
    site.schedules.set("en.gone", schedule).await.unwrap();

    let ran = site.door(10).run_due().await.unwrap();
    let settled = [ID, "en.gone"].map(|id| Ran::Settled(id.to_owned()));
    assert_eq!(ran, settled);
    assert_eq!(site.told(), Vec::<String>::new());
    assert_eq!(site.schedules.due(at(23)).await.unwrap(), []);
}

/// A store whose publish gate refuses every post.
#[cfg(test)]
struct Refusing(MemoryPostStore);

#[cfg(test)]
impl PostStore for Refusing {
    fn get<'a>(&'a self, id: &'a str) -> BoxFuture<'a, Result<Option<AdminPost>, PostStoreError>> {
        self.0.get(id)
    }

    fn list<'a>(
        &'a self,
        query: &'a ListQuery,
    ) -> BoxFuture<'a, Result<Listed<PostRow>, PostStoreError>> {
        self.0.list(query)
    }

    fn update<'a>(
        &'a self,
        id: &'a str,
        edit: PostEdit,
        by: &'a str,
    ) -> BoxFuture<'a, Result<AdminPost, UpdatePostError>> {
        if edit.publishes() {
            let refused = UpdatePostError::Refused(Report::default());
            return Box::pin(std::future::ready(Err(refused)));
        }
        self.0.update(id, edit, by)
    }

    fn create<'a>(
        &'a self,
        draft: PostDraft,
        by: &'a str,
        key: Option<&'a str>,
    ) -> BoxFuture<'a, Result<AdminPost, CreatePostError>> {
        self.0.create(draft, by, key)
    }

    fn delete<'a>(
        &'a self,
        id: &'a str,
        updated_at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<AdminPost, DeletePostError>> {
        self.0.delete(id, updated_at)
    }
}

#[tokio::test]
async fn a_post_the_linter_refuses_stays_a_draft_and_is_not_tried_again() {
    let site = Site::new(Refusing(draft()));
    let schedule = Schedule::default().publishing_at(at(10));
    site.schedules.set(ID, schedule).await.unwrap();

    assert_eq!(
        site.door(10).run_due().await.unwrap(),
        [Ran::Refused(ID.to_owned())]
    );
    assert_eq!(site.door(11).run_due().await.unwrap(), []);
    let post = site.door(11).get(ID).await.unwrap().unwrap();
    assert_eq!(post.status(), PostStatus::Draft);
    assert_eq!(
        site.told(),
        [format!("blocked {ID}")],
        "lint.blocked fires once, and nothing went live"
    );
}
