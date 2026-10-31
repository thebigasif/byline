//! What the analytics routes read: the views `khalid serve` counted, and
//! the clock that says which day it is (GEO M1, KH-GRO-4).

use std::sync::Arc;

use chrono::NaiveDate;
use khalid_analytics::{
    AiVisitors, AnalyticsError, AnalyticsStore, BoxFuture, Crawls, Period, Tally, Uncited, Views,
    Visitors, uncited,
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

    /// The page views in `period`, per day, source and page.
    ///
    /// # Errors
    ///
    /// [`AnalyticsError`] when the views cannot be read.
    pub async fn visitors(&self, period: Period) -> Result<Visitors, AnalyticsError> {
        Visitors::read(self.store.as_ref(), period).await
    }

    /// The pages AI bots read in `period` that no AI assistant sent a
    /// reader to in it, the most read first (GEO M2).
    ///
    /// # Errors
    ///
    /// [`AnalyticsError`] when the bot reads or the views cannot be read.
    pub async fn uncited(&self, period: Period) -> Result<Vec<Uncited>, AnalyticsError> {
        let (first, last) = (period.first(), period.last());
        let crawls = self.store.crawls(first, last).await?;
        let views = self.store.views(first, last).await?;
        Ok(uncited(&crawls, &views))
    }
}

impl std::fmt::Debug for AnalyticsDesk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnalyticsDesk").finish_non_exhaustive()
    }
}

/// Views and bot reads held in memory and read back as given. Saving keeps
/// nothing, so it suits a site that counts nothing, and tests.
#[derive(Debug, Clone, Default)]
pub struct ViewsInMemory {
    views: Vec<Views>,
    crawls: Vec<Crawls>,
}

impl ViewsInMemory {
    /// A store holding `views` and no bot reads.
    pub fn new(views: Vec<Views>) -> Self {
        Self {
            views,
            crawls: Vec::new(),
        }
    }

    /// The same store, holding `crawls` as its bot reads.
    #[must_use]
    pub fn with_crawls(self, crawls: Vec<Crawls>) -> Self {
        Self { crawls, ..self }
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
            .views
            .iter()
            .filter(|row| (first..=last).contains(&row.day));
        let views = held.cloned().collect();
        Box::pin(async move { Ok(views) })
    }

    fn crawls(
        &self,
        first: NaiveDate,
        last: NaiveDate,
    ) -> BoxFuture<'_, Result<Vec<Crawls>, AnalyticsError>> {
        let held = self
            .crawls
            .iter()
            .filter(|row| (first..=last).contains(&row.day));
        let crawls = held.cloned().collect();
        Box::pin(async move { Ok(crawls) })
    }
}
