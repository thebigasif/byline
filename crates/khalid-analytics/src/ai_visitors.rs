//! AI visitors per tool (GEO M1): how many readers each AI assistant sent
//! in a period, against the period before, and the pages they landed on.

use std::collections::BTreeMap;

use chrono::{Days, NaiveDate};

use crate::{AiTool, AnalyticsError, AnalyticsStore, ReferrerGroup, Views};

/// The days a report covers, both ends included.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Period {
    first: NaiveDate,
    last: NaiveDate,
}

impl Period {
    /// The longest period a report covers: a year and a day, so a leap year
    /// fits.
    pub const MOST_DAYS: u32 = 366;

    /// The `days` days ending on `last`, or `None` when `days` is 0 or more
    /// than [`Period::MOST_DAYS`].
    pub fn ending(last: NaiveDate, days: u32) -> Option<Self> {
        if !(1..=Self::MOST_DAYS).contains(&days) {
            return None;
        }
        let first = last.checked_sub_days(Days::new(u64::from(days - 1)))?;
        Some(Self { first, last })
    }

    /// The first day.
    pub fn first(self) -> NaiveDate {
        self.first
    }

    /// The last day.
    pub fn last(self) -> NaiveDate {
        self.last
    }

    /// How many days it covers.
    pub fn days(self) -> u32 {
        let days = (self.last - self.first).num_days().saturating_add(1);
        u32::try_from(days).unwrap_or(Self::MOST_DAYS)
    }

    /// The period of the same length that ends the day before this one
    /// starts. At the start of the calendar, the same period.
    #[must_use]
    pub fn before(self) -> Self {
        let days = Days::new(u64::from(self.days()));
        let earlier = self
            .last
            .checked_sub_days(days)
            .zip(self.first.checked_sub_days(days));
        earlier.map_or(self, |(last, first)| Self { first, last })
    }

    pub(crate) fn holds(self, day: NaiveDate) -> bool {
        (self.first..=self.last).contains(&day)
    }
}

/// The views of one page from one tool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageViews {
    /// The page's path, such as `/blog/internal-links/`.
    pub path: String,
    /// How many views.
    pub views: u64,
}

/// What one AI assistant sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolVisitors {
    /// The assistant.
    pub tool: AiTool,
    /// Views from its readers in the period.
    pub views: u64,
    /// Views from its readers in the period of the same length before.
    pub before: u64,
    /// The pages its readers landed on, most views first, at most
    /// [`AiVisitors::TOP_PAGES`].
    pub pages: Vec<PageViews>,
}

/// The readers AI assistants sent in a period, per tool.
///
/// The counts are views: a reader who opens two pages from one answer
/// counts twice. An AI bot fetching a page is never a view here.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct AiVisitors {
    /// The days counted.
    pub period: Period,
    /// Every view that came from outside the site, AI search included.
    pub arrivals: u64,
    /// The views AI assistants sent.
    pub ai_views: u64,
    /// Each tool that sent readers in the period or the one before, most
    /// views first.
    pub tools: Vec<ToolVisitors>,
}

/// One tool's counts while the report is built.
#[derive(Debug, Default)]
struct Counting {
    views: u64,
    before: u64,
    pages: BTreeMap<String, u64>,
}

impl AiVisitors {
    /// The most pages listed per tool.
    pub const TOP_PAGES: usize = 10;

    /// The report for `period`, read from `store` with the period before.
    ///
    /// # Errors
    ///
    /// [`AnalyticsError`] when the views cannot be read.
    pub async fn read(store: &dyn AnalyticsStore, period: Period) -> Result<Self, AnalyticsError> {
        let views = store.views(period.before().first, period.last).await?;
        Ok(Self::from_views(period, &views))
    }

    /// The report for `period` from `views`. Views outside the period and
    /// the one before it are left out.
    pub fn from_views(period: Period, views: &[Views]) -> Self {
        let before = period.before();
        let (mut arrivals, mut ai_views) = (0_u64, 0_u64);
        let mut tools: BTreeMap<AiTool, Counting> = BTreeMap::new();
        for row in views {
            let now = period.holds(row.day);
            if now && row.group != ReferrerGroup::Internal {
                arrivals = arrivals.saturating_add(row.views);
            }
            let ReferrerGroup::Ai(tool) = row.group else {
                continue;
            };
            if now {
                ai_views = ai_views.saturating_add(row.views);
                let counting = tools.entry(tool).or_default();
                counting.views = counting.views.saturating_add(row.views);
                let page = counting.pages.entry(row.path.clone()).or_default();
                *page = page.saturating_add(row.views);
            } else if before.holds(row.day) {
                let counting = tools.entry(tool).or_default();
                counting.before = counting.before.saturating_add(row.views);
            }
        }
        let mut tools: Vec<ToolVisitors> = tools.into_iter().map(finished).collect();
        // Stable, so ties keep the tools' own order.
        tools.sort_by_key(|tool| std::cmp::Reverse(tool.views));
        Self {
            period,
            arrivals,
            ai_views,
            tools,
        }
    }
}

fn finished((tool, counting): (AiTool, Counting)) -> ToolVisitors {
    let mut pages: Vec<PageViews> = counting
        .pages
        .into_iter()
        .map(|(path, views)| PageViews { path, views })
        .collect();
    pages.sort_by_key(|page| std::cmp::Reverse(page.views));
    pages.truncate(AiVisitors::TOP_PAGES);
    ToolVisitors {
        tool,
        views: counting.views,
        before: counting.before,
        pages,
    }
}
