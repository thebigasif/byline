//! Where the publish door keeps each post's schedule: when it goes live,
//! and when it comes down (CTO answers 2026-10-27, section 4).

use std::error::Error;

use chrono::{DateTime, Utc};

use crate::content_store::BoxFuture;

/// When a post goes live and when it comes down. Either may be unset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Schedule {
    publish_at: Option<DateTime<Utc>>,
    unpublish_at: Option<DateTime<Utc>>,
}

impl Schedule {
    /// The same schedule, going live at `at`.
    #[must_use]
    pub fn publishing_at(self, at: DateTime<Utc>) -> Self {
        let publish_at = Some(at);
        Self { publish_at, ..self }
    }

    /// The same schedule, coming down at `at`.
    #[must_use]
    pub fn unpublishing_at(self, at: DateTime<Utc>) -> Self {
        let unpublish_at = Some(at);
        Self {
            unpublish_at,
            ..self
        }
    }

    /// When the post goes live, if it is scheduled to.
    pub fn publish_at(&self) -> Option<DateTime<Utc>> {
        self.publish_at
    }

    /// When the post comes down, if it is scheduled to.
    pub fn unpublish_at(&self) -> Option<DateTime<Utc>> {
        self.unpublish_at
    }

    /// Whether nothing is scheduled.
    pub fn is_empty(&self) -> bool {
        self.publish_at.is_none() && self.unpublish_at.is_none()
    }
}

/// What a due schedule asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Due {
    /// The post's publish time has come.
    Publish,
    /// The post's unpublish time has come.
    Unpublish,
}

/// Each post's schedule. `khalid-db` keeps it in SQLite, so it outlasts a
/// restart; `MemoryScheduleStore`, from the `test-util` feature, keeps it
/// in memory for tests.
pub trait ScheduleStore: Send + Sync {
    /// The schedule of post `id`, empty when it has none.
    ///
    /// # Errors
    ///
    /// Returns [`ScheduleStoreError`] when the schedules cannot be read.
    fn get<'a>(&'a self, id: &'a str) -> BoxFuture<'a, Result<Schedule, ScheduleStoreError>>;

    /// Gives post `id` the schedule `schedule`; an empty one clears it.
    ///
    /// # Errors
    ///
    /// Returns [`ScheduleStoreError`] when the schedules cannot be written.
    fn set<'a>(
        &'a self,
        id: &'a str,
        schedule: Schedule,
    ) -> BoxFuture<'a, Result<(), ScheduleStoreError>>;

    /// Every time that has come by `now`, oldest first, with the identity
    /// of its post.
    ///
    /// # Errors
    ///
    /// Returns [`ScheduleStoreError`] when the schedules cannot be read.
    fn due(
        &self,
        now: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<Vec<(String, Due)>, ScheduleStoreError>>;
}

/// The schedules could not be read or written.
#[derive(Debug, thiserror::Error)]
#[error("the schedules could not be read or written")]
pub struct ScheduleStoreError {
    #[source]
    source: Box<dyn Error + Send + Sync>,
}

impl ScheduleStoreError {
    /// Wraps what the storage reported.
    pub fn new(source: impl Error + Send + Sync + 'static) -> Self {
        Self {
            source: Box::new(source),
        }
    }
}

#[cfg(any(test, feature = "test-util"))]
pub use memory::MemoryScheduleStore;

#[cfg(any(test, feature = "test-util"))]
mod memory {
    use std::collections::BTreeMap;
    use std::sync::{Mutex, PoisonError};

    use chrono::{DateTime, Utc};

    use super::{BoxFuture, Due, Schedule, ScheduleStore, ScheduleStoreError};

    /// Schedules held in memory, for tests.
    #[derive(Debug, Default)]
    pub struct MemoryScheduleStore(Mutex<BTreeMap<String, Schedule>>);

    impl MemoryScheduleStore {
        fn due_now(&self, now: DateTime<Utc>) -> Vec<(String, Due)> {
            let schedules = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            let mut due: Vec<(DateTime<Utc>, String, Due)> = Vec::new();
            for (id, schedule) in schedules.iter() {
                let times = [
                    (schedule.publish_at, Due::Publish),
                    (schedule.unpublish_at, Due::Unpublish),
                ];
                for (time, what) in times {
                    if let Some(time) = time.filter(|time| *time <= now) {
                        due.push((time, id.clone(), what));
                    }
                }
            }
            due.sort_by_key(|(time, ..)| *time);
            due.into_iter().map(|(_, id, what)| (id, what)).collect()
        }
    }

    /// Memory answers at once, so each future is ready when returned.
    impl ScheduleStore for MemoryScheduleStore {
        fn get<'a>(&'a self, id: &'a str) -> BoxFuture<'a, Result<Schedule, ScheduleStoreError>> {
            let schedules = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            let found = schedules.get(id).copied().unwrap_or_default();
            Box::pin(std::future::ready(Ok(found)))
        }

        fn set<'a>(
            &'a self,
            id: &'a str,
            schedule: Schedule,
        ) -> BoxFuture<'a, Result<(), ScheduleStoreError>> {
            let mut schedules = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            if schedule.is_empty() {
                schedules.remove(id);
            } else {
                schedules.insert(id.to_owned(), schedule);
            }
            Box::pin(std::future::ready(Ok(())))
        }

        fn due(
            &self,
            now: DateTime<Utc>,
        ) -> BoxFuture<'_, Result<Vec<(String, Due)>, ScheduleStoreError>> {
            Box::pin(std::future::ready(Ok(self.due_now(now))))
        }
    }
}
