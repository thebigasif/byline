//! What the analytics routes read: the views `khalid serve` counted, and
//! the clock that says which day it is (GEO M1).

use std::sync::Arc;

use chrono::NaiveDate;
use khalid_analytics::{
    AiVisitors, AnalyticsError, AnalyticsStore, BoxFuture, Crawls, Period, Tally, Views,
};
use khalid_core::{Clock, SystemClock};

/// The site's counted views and its clock.
#[derive(Clone)]
pub struct AnalyticsDesk {
    store: Arc<dyn AnalyticsStore>,
    clock: Arc<dyn Clock>,
}

impl AnalyticsDesk {
    /// The views in `store`, with today from `clock`.
    pub fn new(store: Arc<dyn AnalyticsStore>, clock: Arc<dyn Clock>) -> Self {
        Self { store, clock }
    }

    /// A site that counts nothing, such as one built to static files.
    pub fn none() -> Self {
        Self::new(Arc::new(ViewsInMemory::default()), Arc::new(SystemClock))
    }

    /// Today, in UTC.
    pub fn today(&self) -> NaiveDate {
        self.clock.now().date_naive()
    }

    /// The readers each AI assistant sent in `period`.
    ///
    /// # Errors
    ///
    /// [`AnalyticsError`] when the views cannot be read.
    pub async fn ai_visitors(&self, period: Period) -> Result<AiVisitors, AnalyticsError> {
        AiVisitors::read(self.store.as_ref(), period).await
    }
}

impl std::fmt::Debug for AnalyticsDesk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnalyticsDesk").finish_non_exhaustive()
    }
}

/// Views held in memory and read back as given. Saving keeps nothing, so
/// it suits a site that counts nothing, and tests.
#[derive(Debug, Clone, Default)]
pub struct ViewsInMemory(Vec<Views>);

impl ViewsInMemory {
    /// A store holding `views`.
    pub fn new(views: Vec<Views>) -> Self {
        Self(views)
    }
}

impl AnalyticsStore for ViewsInMemory {
    fn save<'a>(&'a self, _: &'a Tally) -> BoxFuture<'a, Result<(), AnalyticsError>> {
        Box::pin(async { Ok(()) })
    }

    fn views(
        &self,
        first: NaiveDate,
        last: NaiveDate,
    ) -> BoxFuture<'_, Result<Vec<Views>, AnalyticsError>> {
        let held = self
            .0
            .iter()
            .filter(|row| (first..=last).contains(&row.day));
        let views = held.cloned().collect();
        Box::pin(async move { Ok(views) })
    }

    fn crawls(
        &self,
        _: NaiveDate,
        _: NaiveDate,
    ) -> BoxFuture<'_, Result<Vec<Crawls>, AnalyticsError>> {
        Box::pin(async { Ok(Vec::new()) })
    }
}
