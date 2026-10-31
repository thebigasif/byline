//! The visitor counts (KH-GRO-4): page views per day, per source and per
//! page in a period, against the period before.

use std::collections::BTreeMap;

use chrono::NaiveDate;

use crate::{AnalyticsError, AnalyticsStore, PageViews, Period, ReferrerGroup, Views};

/// Where views came from, with every AI assistant together. The finer
/// split per assistant is [`crate::AiVisitors`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Source {
    /// No referrer: a typed address, a bookmark, or an app that sends none.
    Direct,
    /// A search engine.
    Search,
    /// A social site or link aggregator.
    Social,
    /// An AI assistant, any of them.
    Ai,
    /// Any other site.
    Other,
    /// Another page of the same site.
    Internal,
}

impl Source {
    /// Every source, those from outside the site first.
    pub const ALL: [Self; 6] = [
        Self::Direct,
        Self::Search,
        Self::Social,
        Self::Ai,
        Self::Other,
        Self::Internal,
    ];

    /// The source of views counted under `group`.
    pub fn of(group: ReferrerGroup) -> Self {
        match group {
            ReferrerGroup::Direct => Self::Direct,
            ReferrerGroup::Internal => Self::Internal,
            ReferrerGroup::Search => Self::Search,
            ReferrerGroup::Social => Self::Social,
            ReferrerGroup::Ai(_) => Self::Ai,
            ReferrerGroup::Other => Self::Other,
        }
    }

    /// The name programs read, such as `search` or `ai`.
    pub fn key(self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::Search => "search",
            Self::Social => "social",
            Self::Ai => "ai",
            Self::Other => "other",
            Self::Internal => "internal",
        }
    }
}

/// The views from one source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceViews {
    /// Where they came from.
    pub source: Source,
    /// Views in the period.
    pub views: u64,
    /// Views in the period of the same length before.
    pub before: u64,
}

/// The views on one day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DayViews {
    /// The day, in UTC.
    pub day: NaiveDate,
    /// Views that day.
    pub views: u64,
}

/// The page views `khalid serve` counted in a period.
///
/// A view is one page opened by a person: a reader who opens two pages
/// counts twice, and a bot never counts.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Visitors {
    /// The days counted.
    pub period: Period,
    /// Every view in the period.
    pub views: u64,
    /// Every view in the period of the same length before.
    pub before: u64,
    /// The views in the period that came from outside the site.
    pub arrivals: u64,
    /// Each source in [`Source::ALL`] order, even one with no views.
    pub sources: Vec<SourceViews>,
    /// Each day of the period, first day first, even one with no views.
    pub days: Vec<DayViews>,
    /// The most viewed pages, most views first, at most
    /// [`Visitors::TOP_PAGES`].
    pub pages: Vec<PageViews>,
}

impl Visitors {
    /// The most pages listed.
    pub const TOP_PAGES: usize = 20;

    /// The counts for `period`, read from `store` with the period before.
    ///
    /// # Errors
    ///
    /// [`AnalyticsError`] when the views cannot be read.
    pub async fn read(store: &dyn AnalyticsStore, period: Period) -> Result<Self, AnalyticsError> {
        let views = store.views(period.before().first(), period.last()).await?;
        Ok(Self::from_views(period, &views))
    }

    /// The counts for `period` from `views`. Views outside the period and
    /// the one before it are left out.
    pub fn from_views(period: Period, views: &[Views]) -> Self {
        let before_period = period.before();
        let mut sources: BTreeMap<Source, SourceViews> = Source::ALL
            .into_iter()
            .map(|source| {
                (
                    source,
                    SourceViews {
                        source,
                        views: 0,
                        before: 0,
                    },
                )
            })
            .collect();
        let mut days: BTreeMap<NaiveDate, u64> = period
            .first()
            .iter_days()
            .take_while(|day| *day <= period.last())
            .map(|day| (day, 0))
            .collect();
        let mut pages: BTreeMap<&str, u64> = BTreeMap::new();
        for row in views {
            // Every source is in the map, so this always finds one.
            let Some(source) = sources.get_mut(&Source::of(row.group)) else {
                continue;
            };
            if let Some(day) = days.get_mut(&row.day) {
                source.views = source.views.saturating_add(row.views);
                *day = day.saturating_add(row.views);
                let page = pages.entry(&row.path).or_default();
                *page = page.saturating_add(row.views);
            } else if before_period.holds(row.day) {
                source.before = source.before.saturating_add(row.views);
            }
        }
        let sources: Vec<SourceViews> = sources.into_values().collect();
        let total =
            |of: fn(&SourceViews) -> u64| sources.iter().map(of).fold(0_u64, u64::saturating_add);
        let mut pages: Vec<PageViews> = pages
            .into_iter()
            .map(|(path, views)| PageViews {
                path: path.to_owned(),
                views,
            })
            .collect();
        // Stable, so ties keep the paths' own order.
        pages.sort_by_key(|page| std::cmp::Reverse(page.views));
        pages.truncate(Self::TOP_PAGES);
        Self {
            period,
            views: total(|row| row.views),
            before: total(|row| row.before),
            arrivals: total(|row| {
                if row.source == Source::Internal {
                    0
                } else {
                    row.views
                }
            }),
            sources,
            days: days
                .into_iter()
                .map(|(day, views)| DayViews { day, views })
                .collect(),
            pages,
        }
    }
}
