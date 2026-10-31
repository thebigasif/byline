//! What the publishing planner knows of a post, and the pace it plans at.

use std::num::NonZeroU8;

use chrono::{Days, NaiveDate};
use khalid_core::Slug;

/// The most posts a week a plan spaces out: two a day.
const MOST_PER_WEEK: u8 = 14;

/// How many weeks of live posts set the pace when none is asked for:
/// twelve, so one quiet or busy week moves it little.
const HISTORY_WEEKS: u64 = 12;

/// A post's role in its topic cluster.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ClusterRole {
    /// The main post, which covers the whole subject.
    Pillar,
    /// A post on one part of the subject, which links to the pillar.
    Member,
}

/// Where a post stands, as far as the planner cares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PlanState {
    /// Live, dated this day.
    Live(NaiveDate),
    /// A draft that goes live by itself on this day.
    Scheduled(NaiveDate),
    /// A draft the lint publish gate lets through.
    Ready,
    /// A draft the lint publish gate refuses.
    Blocked,
}

/// One post the planner looks at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanPost {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) state: PlanState,
    pub(crate) cluster: Option<(Slug, ClusterRole)>,
}

impl PlanPost {
    /// Post `id`, such as `en.internal-links`, headed `title`, standing at
    /// `state`, in no cluster.
    pub fn new(id: impl Into<String>, title: impl Into<String>, state: PlanState) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            state,
            cluster: None,
        }
    }

    /// The same post, in cluster `name` with `role`.
    #[must_use]
    pub fn in_cluster(self, name: Slug, role: ClusterRole) -> Self {
        Self {
            cluster: Some((name, role)),
            ..self
        }
    }

    pub(crate) fn is_pillar(&self) -> bool {
        matches!(self.cluster, Some((_, ClusterRole::Pillar)))
    }
}

/// How many posts a week the plan spaces out, from 1 to 14.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pace(NonZeroU8);

impl Pace {
    /// A pace of `posts` a week, or `None` outside 1 to 14.
    pub fn per_week(posts: u8) -> Option<Self> {
        NonZeroU8::new(posts)
            .filter(|n| n.get() <= MOST_PER_WEEK)
            .map(Self)
    }

    /// The posts a week.
    pub fn get(self) -> u8 {
        self.0.get()
    }

    /// The site's own pace: its live posts of the last twelve weeks, a
    /// week's worth rounded, at least one.
    pub(crate) fn of(posts: &[PlanPost], today: NaiveDate) -> Self {
        let from = today.checked_sub_days(Days::new(HISTORY_WEEKS * 7 - 1));
        let live = live_between(posts, from, today);
        let live = u64::try_from(live).unwrap_or(u64::MAX);
        let weekly = live.saturating_add(HISTORY_WEEKS / 2) / HISTORY_WEEKS;
        let weekly = u8::try_from(weekly).unwrap_or(MOST_PER_WEEK);
        Self::per_week(weekly.clamp(1, MOST_PER_WEEK)).unwrap_or(Self(NonZeroU8::MIN))
    }
}

/// Where the plan's pace came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PaceFrom {
    /// The person asked for it.
    Asked,
    /// The site's live posts of the last twelve weeks.
    History,
}

/// The live posts dated from `from`, or the beginning when `None`, to
/// `to`, both included.
pub(crate) fn live_between(posts: &[PlanPost], from: Option<NaiveDate>, to: NaiveDate) -> usize {
    posts
        .iter()
        .filter(|post| match post.state {
            PlanState::Live(day) => from.is_none_or(|from| day >= from) && day <= to,
            _ => false,
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn live(on: &str) -> PlanPost {
        PlanPost::new("en.a", "A", PlanState::Live(on.parse().unwrap()))
    }

    #[test]
    fn the_pace_is_one_to_fourteen_a_week() {
        assert_eq!(Pace::per_week(0), None);
        assert_eq!(Pace::per_week(15), None);
        assert_eq!(Pace::per_week(14).map(Pace::get), Some(14));
    }

    #[test]
    fn the_site_pace_counts_twelve_weeks_of_live_posts() {
        let mut posts: Vec<PlanPost> = (0..24).map(|_| live("2026-10-01")).collect();
        posts.push(live("2025-01-01"));
        posts.push(PlanPost::new("en.b", "B", PlanState::Ready));
        assert_eq!(Pace::of(&posts, "2026-11-02".parse().unwrap()).get(), 2);
    }

    #[test]
    fn a_site_with_no_history_has_a_pace_of_one() {
        assert_eq!(Pace::of(&[], "2026-11-02".parse().unwrap()).get(), 1);
    }
}
