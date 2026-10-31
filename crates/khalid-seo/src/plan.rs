//! The publishing planner (GEO task M4): which ready draft goes out on
//! which day, at a steady pace, with each cluster's pillar before its
//! members.
//!
//! The planner only suggests. It writes nothing and never moves a date
//! (decision D4): a post goes live through the publish door, and a
//! schedule is set there too.

use std::collections::BTreeMap;

use chrono::{Days, NaiveDate};
use khalid_core::Slug;

use crate::plan_post::{ClusterRole, Pace, PaceFrom, PlanPost, PlanState};

/// Whether a day on the calendar is already set or only suggested.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SlotKind {
    /// The post is scheduled for the day.
    Scheduled,
    /// The planner suggests the day; nothing is set.
    Suggested,
}

/// One post on one day of the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Slot {
    /// The day.
    pub day: NaiveDate,
    /// The post's identity.
    pub id: String,
    /// The post's title.
    pub title: String,
    /// Set or suggested.
    pub kind: SlotKind,
}

/// Why a draft has no day in the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum WaitReason {
    /// The lint publish gate refuses it.
    Blocked,
    /// It is a member and its cluster's pillar, this post, is refused.
    PillarBlocked(String),
}

/// A draft with no day, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Waiting {
    /// The post's identity.
    pub id: String,
    /// The post's title.
    pub title: String,
    /// Why it waits.
    pub why: WaitReason,
}

/// The plan: the pace, the calendar from tomorrow, and the drafts that
/// wait.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Plan {
    /// Posts a week.
    pub pace: Pace,
    /// Where the pace came from.
    pub pace_from: PaceFrom,
    /// Scheduled and suggested posts, by day.
    pub calendar: Vec<Slot>,
    /// Drafts with no day.
    pub waiting: Vec<Waiting>,
}

/// Plans `posts` from the day after `today`, at `pace` or else the site's
/// own pace. Each ready draft gets the next free day at the pace; a day
/// already holding a scheduled post is taken. Pillars go first, and a
/// member never goes before its cluster's pillar.
pub fn plan(posts: &[PlanPost], today: NaiveDate, pace: Option<Pace>) -> Plan {
    let (pace, pace_from) = match pace {
        Some(pace) => (pace, PaceFrom::Asked),
        None => (Pace::of(posts, today), PaceFrom::History),
    };
    let pillars = pillars(posts);
    let mut calendar: Vec<Slot> = posts
        .iter()
        .filter_map(|post| match post.state {
            PlanState::Scheduled(day) => Some(slot(post, day, SlotKind::Scheduled)),
            _ => None,
        })
        .collect();
    let mut waiting = Vec::new();
    let mut queue = Vec::new();
    for post in posts {
        match post.state {
            PlanState::Blocked => waiting.push(wait(post, WaitReason::Blocked)),
            PlanState::Ready => match pillar_of(post, &pillars) {
                Some(pillar) if pillar.state == PlanState::Blocked => {
                    waiting.push(wait(post, WaitReason::PillarBlocked(pillar.id.clone())));
                }
                _ => queue.push(post),
            },
            PlanState::Live(_) | PlanState::Scheduled(_) => {}
        }
    }
    queue.sort_by_key(|post| (!post.is_pillar(), post.id.as_str()));
    let suggested = suggest(&queue, &calendar, &pillars, (today, pace));
    calendar.extend(suggested);
    calendar.sort_by(|a, b| (a.day, &a.id).cmp(&(b.day, &b.id)));
    Plan {
        pace,
        pace_from,
        calendar,
        waiting,
    }
}

/// Each cluster's pillar: the first post that says it is one.
fn pillars(posts: &[PlanPost]) -> BTreeMap<&Slug, &PlanPost> {
    let mut pillars = BTreeMap::new();
    for post in posts {
        if let Some((name, ClusterRole::Pillar)) = &post.cluster {
            pillars.entry(name).or_insert(post);
        }
    }
    pillars
}

/// The pillar of a member's cluster, if the member has one other than
/// itself.
fn pillar_of<'a>(post: &PlanPost, pillars: &BTreeMap<&Slug, &'a PlanPost>) -> Option<&'a PlanPost> {
    let (name, ClusterRole::Member) = post.cluster.as_ref()? else {
        return None;
    };
    pillars.get(name).copied()
}

/// Gives each post in `queue`, in order, the next day at `pace` after
/// `today` that is not taken and comes after its pillar's day.
fn suggest(
    queue: &[&PlanPost],
    calendar: &[Slot],
    pillars: &BTreeMap<&Slug, &PlanPost>,
    (today, pace): (NaiveDate, Pace),
) -> Vec<Slot> {
    let mut taken: BTreeMap<NaiveDate, usize> = BTreeMap::new();
    let mut days: BTreeMap<&str, NaiveDate> = BTreeMap::new();
    for slot in calendar {
        *taken.entry(slot.day).or_default() += 1;
        days.insert(slot.id.as_str(), slot.day);
    }
    let mut left: Vec<&PlanPost> = queue.to_vec();
    let mut suggested = Vec::new();
    let mut k: u64 = 0;
    while !left.is_empty() {
        let offset = 1 + k * 7 / u64::from(pace.get());
        k += 1;
        let Some(day) = today.checked_add_days(Days::new(offset)) else {
            break;
        };
        if let Some(count) = taken.get_mut(&day).filter(|count| **count > 0) {
            *count -= 1;
            continue;
        }
        let after_pillar = |post: &&PlanPost| {
            pillar_of(post, pillars)
                .and_then(|pillar| days.get(pillar.id.as_str()))
                .is_none_or(|pillar_day| *pillar_day < day)
        };
        let Some(at) = left.iter().position(after_pillar) else {
            continue;
        };
        let post = left.remove(at);
        days.insert(post.id.as_str(), day);
        suggested.push(slot(post, day, SlotKind::Suggested));
    }
    suggested
}

fn slot(post: &PlanPost, day: NaiveDate, kind: SlotKind) -> Slot {
    Slot {
        day,
        id: post.id.clone(),
        title: post.title.clone(),
        kind,
    }
}

fn wait(post: &PlanPost, why: WaitReason) -> Waiting {
    Waiting {
        id: post.id.clone(),
        title: post.title.clone(),
        why,
    }
}

#[cfg(test)]
#[path = "plan_tests.rs"]
mod tests;
