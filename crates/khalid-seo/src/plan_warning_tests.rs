use chrono::NaiveDate;
use khalid_core::Slug;

use super::*;
use crate::plan::plan;
use crate::plan_post::{ClusterRole, Pace};

fn day(text: &str) -> NaiveDate {
    text.parse().unwrap()
}

fn cluster() -> Slug {
    Slug::try_new("internal-links").unwrap()
}

fn ready(id: &str) -> PlanPost {
    PlanPost::new(id, id, PlanState::Ready)
}

fn live(id: &str, on: &str) -> PlanPost {
    PlanPost::new(id, id, PlanState::Live(day(on)))
}

fn warned(posts: &[PlanPost]) -> Vec<PlanWarning> {
    plan(posts, day("2026-11-02"), Pace::per_week(7)).warnings
}

#[test]
fn a_member_scheduled_before_its_pillar_is_warned_about() {
    let pillar = ready("en.pillar").in_cluster(cluster(), ClusterRole::Pillar);
    let member = PlanPost::new(
        "en.member",
        "Member",
        PlanState::Scheduled(day("2026-11-03")),
    )
    .in_cluster(cluster(), ClusterRole::Member);
    assert_eq!(
        warned(&[pillar, member]),
        [PlanWarning::MemberBeforePillar {
            member: "en.member".to_owned(),
            pillar: "en.pillar".to_owned(),
        }]
    );
}

#[test]
fn members_after_their_pillar_are_fine() {
    let posts = [
        ready("en.a-member").in_cluster(cluster(), ClusterRole::Member),
        ready("en.z-pillar").in_cluster(cluster(), ClusterRole::Pillar),
        live("en.old-pillar", "2026-06-01")
            .in_cluster(Slug::try_new("anchors").unwrap(), ClusterRole::Pillar),
        ready("en.b-member").in_cluster(Slug::try_new("anchors").unwrap(), ClusterRole::Member),
    ];
    assert_eq!(warned(&posts), []);
}

#[test]
fn fewer_live_posts_in_the_last_four_weeks_than_before_is_warned_about() {
    let posts = [
        live("en.a", "2026-10-30"),
        live("en.b", "2026-10-01"),
        live("en.c", "2026-09-20"),
        ready("en.d"),
    ];
    assert_eq!(
        warned(&posts),
        [PlanWarning::PaceDropped {
            recent: 1,
            before: 2
        }]
    );
}

#[test]
fn no_ready_draft_says_when_the_calendar_runs_out() {
    let scheduled = PlanPost::new("en.set", "Set", PlanState::Scheduled(day("2026-11-09")));
    assert_eq!(
        warned(&[scheduled]),
        [PlanWarning::NoneReady {
            last: Some(day("2026-11-09"))
        }]
    );
}
