use chrono::NaiveDate;
use khalid_core::Slug;

use super::*;

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

fn days(plan: &Plan) -> Vec<(String, String)> {
    plan.calendar
        .iter()
        .map(|slot| (slot.id.clone(), slot.day.to_string()))
        .collect()
}

fn pair(id: &str, on: &str) -> (String, String) {
    (id.to_owned(), on.to_owned())
}

#[test]
fn ready_drafts_are_spaced_at_the_asked_pace_from_tomorrow() {
    let posts = [ready("en.a"), ready("en.b"), ready("en.c")];
    let plan = plan(&posts, day("2026-11-02"), Pace::per_week(2));
    assert_eq!(plan.pace_from, PaceFrom::Asked);
    assert_eq!(
        days(&plan),
        [
            pair("en.a", "2026-11-03"),
            pair("en.b", "2026-11-06"),
            pair("en.c", "2026-11-10"),
        ]
    );
    assert!(plan.calendar.iter().all(|s| s.kind == SlotKind::Suggested));
}

#[test]
fn the_pace_comes_from_the_site_when_none_is_asked() {
    let mut posts: Vec<PlanPost> = (1..=24)
        .map(|n| live(&format!("en.old-{n}"), "2026-10-01"))
        .collect();
    posts.push(ready("en.new"));
    let plan = plan(&posts, day("2026-11-02"), None);
    assert_eq!((plan.pace.get(), plan.pace_from), (2, PaceFrom::History));
    assert_eq!(days(&plan), [pair("en.new", "2026-11-03")]);
}

#[test]
fn a_scheduled_day_is_taken_and_the_next_draft_moves_on() {
    let scheduled = PlanPost::new("en.set", "Set", PlanState::Scheduled(day("2026-11-03")));
    let plan = plan(
        &[scheduled, ready("en.a")],
        day("2026-11-02"),
        Pace::per_week(7),
    );
    assert_eq!(
        days(&plan),
        [pair("en.set", "2026-11-03"), pair("en.a", "2026-11-04")]
    );
    assert_eq!(plan.calendar[0].kind, SlotKind::Scheduled);
}

#[test]
fn the_pillar_goes_before_its_members() {
    let posts = [
        ready("en.a-member").in_cluster(cluster(), ClusterRole::Member),
        ready("en.z-pillar").in_cluster(cluster(), ClusterRole::Pillar),
    ];
    let plan = plan(&posts, day("2026-11-02"), Pace::per_week(7));
    assert_eq!(
        days(&plan),
        [
            pair("en.z-pillar", "2026-11-03"),
            pair("en.a-member", "2026-11-04"),
        ]
    );
}

#[test]
fn a_member_waits_for_a_pillar_scheduled_later() {
    let pillar = PlanPost::new(
        "en.pillar",
        "Pillar",
        PlanState::Scheduled(day("2026-11-05")),
    )
    .in_cluster(cluster(), ClusterRole::Pillar);
    let member = ready("en.member").in_cluster(cluster(), ClusterRole::Member);
    let posts = [member, ready("en.other"), pillar];
    let plan = plan(&posts, day("2026-11-02"), Pace::per_week(7));
    assert_eq!(
        days(&plan),
        [
            pair("en.other", "2026-11-03"),
            pair("en.pillar", "2026-11-05"),
            pair("en.member", "2026-11-06"),
        ]
    );
}

#[test]
fn a_member_of_a_live_pillar_goes_at_once() {
    let pillar = live("en.pillar", "2026-06-01").in_cluster(cluster(), ClusterRole::Pillar);
    let member = ready("en.member").in_cluster(cluster(), ClusterRole::Member);
    let plan = plan(&[pillar, member], day("2026-11-02"), Pace::per_week(1));
    assert_eq!(days(&plan), [pair("en.member", "2026-11-03")]);
}

#[test]
fn a_blocked_draft_and_the_members_of_a_blocked_pillar_wait() {
    let pillar = PlanPost::new("en.pillar", "Pillar", PlanState::Blocked)
        .in_cluster(cluster(), ClusterRole::Pillar);
    let member = ready("en.member").in_cluster(cluster(), ClusterRole::Member);
    let plan = plan(&[pillar, member], day("2026-11-02"), None);
    assert_eq!(plan.calendar, Vec::<Slot>::new());
    let why: Vec<_> = plan.waiting.iter().map(|w| w.why.clone()).collect();
    assert_eq!(
        why,
        [
            WaitReason::Blocked,
            WaitReason::PillarBlocked("en.pillar".to_owned()),
        ]
    );
}
