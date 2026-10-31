//! What the publishing planner warns about: a dropped pace, a member
//! before its pillar, and no ready draft.

use std::collections::BTreeMap;

use chrono::{Days, NaiveDate};
use khalid_core::Slug;

use crate::plan::{Slot, pillar_of};
use crate::plan_post::{PlanPost, PlanState, live_between};

/// The length of each of the two windows the pace warning compares: the
/// last four weeks against the four before them.
const WINDOW_DAYS: u64 = 28;

/// Something about the plan the person should act on.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PlanWarning {
    /// Fewer posts went live in the last four weeks than in the four before.
    PaceDropped {
        /// Live posts in the last four weeks.
        recent: usize,
        /// Live posts in the four weeks before.
        before: usize,
    },
    /// A member goes live before its pillar, or the pillar has no day.
    MemberBeforePillar {
        /// The member's identity.
        member: String,
        /// The pillar's identity.
        pillar: String,
    },
    /// No draft is ready, so the pace stops after the last scheduled day,
    /// or at once when nothing is scheduled.
    NoneReady {
        /// The last scheduled day, if any.
        last: Option<NaiveDate>,
    },
}

/// The warnings for `posts`, whose plan from `today` is `calendar`.
pub(crate) fn warnings(
    posts: &[PlanPost],
    calendar: &[Slot],
    pillars: &BTreeMap<&Slug, &PlanPost>,
    today: NaiveDate,
) -> Vec<PlanWarning> {
    let mut warnings = Vec::new();
    let window = |back: u64| today.checked_sub_days(Days::new(back));
    let recent = live_between(posts, window(WINDOW_DAYS - 1), today);
    let before = window(WINDOW_DAYS).map_or(0, |end| {
        live_between(posts, window(2 * WINDOW_DAYS - 1), end)
    });
    if recent < before {
        warnings.push(PlanWarning::PaceDropped { recent, before });
    }
    let day_of = |id: &str| calendar.iter().find(|slot| slot.id == id).map(|s| s.day);
    for post in posts {
        let (Some(pillar), Some(day)) = (pillar_of(post, pillars), day_of(&post.id)) else {
            continue;
        };
        let pillar_first = match pillar.state {
            PlanState::Live(_) => true,
            _ => day_of(&pillar.id).is_some_and(|pillar_day| pillar_day < day),
        };
        if !pillar_first {
            warnings.push(PlanWarning::MemberBeforePillar {
                member: post.id.clone(),
                pillar: pillar.id.clone(),
            });
        }
    }
    if !posts.iter().any(|post| post.state == PlanState::Ready) {
        let last = calendar.iter().map(|slot| slot.day).max();
        warnings.push(PlanWarning::NoneReady { last });
    }
    warnings
}

#[cfg(test)]
#[path = "plan_warning_tests.rs"]
mod tests;
