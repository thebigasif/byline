//! `khalid plan`: the publishing planner (GEO task M4). Suggests a day for
//! each ready draft at a steady pace, pillars before their members, and
//! says what to act on. Writes nothing; a post goes live through the
//! publish door, and its schedule is set there.

use std::fmt::Write as _;
use std::path::Path;

use khalid_api::resources::{Gate, PostStatus};
use khalid_api::schedule_store::ScheduleStore;
use khalid_content::{PostFront, TopicRole};
use khalid_core::{Clock, Config, SystemClock, UserMessage};
use khalid_db::ScheduleTable;
use khalid_seo::{
    ClusterRole, Pace, PaceFrom, Plan, PlanPost, PlanState, PlanWarning, SlotKind, WaitReason,
};
use serde::Serialize;

use crate::cli::{Cli, Output};
use crate::exit::Exit;
use crate::site_posts::SitePosts;
use crate::{plan_message, report, serve};

/// The JSON shape of a plan.
#[derive(Serialize)]
struct PlanBody {
    per_week: u8,
    pace_from: &'static str,
    calendar: Vec<SlotBody>,
    waiting: Vec<WaitingBody>,
    warnings: Vec<UserMessage>,
}

#[derive(Serialize)]
struct SlotBody {
    day: String,
    id: String,
    title: String,
    kind: &'static str,
}

#[derive(Serialize)]
struct WaitingBody {
    id: String,
    title: String,
    why: String,
}

/// Plans the checked site in `folder` at `pace`, or at the site's own.
pub(crate) fn run(cli: &Cli, (folder, config): (&Path, &Config), pace: Option<Pace>) -> Exit {
    match plan_site(folder, config, pace) {
        Ok(plan) => match cli.output {
            Output::Json => report::json(&body(&plan), Exit::Success),
            Output::Human => report::text(&text(&plan), Exit::Success),
        },
        Err(message) => report::failure(cli.output, &message, Exit::Failure),
    }
}

/// Reads every post and each draft's schedule, then plans them.
fn plan_site(folder: &Path, config: &Config, pace: Option<Pace>) -> Result<Plan, UserMessage> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| plan_message::unreadable())?;
    let (database, site) = serve::open(folder, &runtime)?;
    let posts = runtime.block_on(async {
        let planned = SitePosts::new(folder, config.clone(), SystemClock)
            .planned()
            .await
            .map_err(|_| plan_message::unreadable())?;
        let schedules = ScheduleTable::new(database.clone(), site);
        let mut posts = Vec::new();
        for one in planned {
            let state = if one.post.status() == PostStatus::Published {
                // A live post with no date is the linter's to report.
                let Some(day) = one.front.date else {
                    continue;
                };
                PlanState::Live(day)
            } else {
                let schedule = schedules.get(one.post.id()).await;
                let schedule = schedule.map_err(|_| plan_message::unreadable())?;
                match (schedule.publish_at(), one.gate) {
                    (Some(at), _) => PlanState::Scheduled(at.date_naive()),
                    (None, Gate::Passes) => PlanState::Ready,
                    (None, _) => PlanState::Blocked,
                }
            };
            posts.push(in_cluster(
                PlanPost::new(one.post.id(), one.post.title(), state),
                &one.front,
            ));
        }
        Ok::<_, UserMessage>(posts)
    });
    runtime.block_on(database.close());
    let today = SystemClock.now().date_naive();
    Ok(khalid_seo::plan(&posts?, today, pace))
}

/// `post` in the topic cluster its front matter names, if any.
fn in_cluster(post: PlanPost, front: &PostFront) -> PlanPost {
    match &front.cluster {
        Some(cluster) => {
            let role = match cluster.role {
                TopicRole::Pillar => ClusterRole::Pillar,
                _ => ClusterRole::Member,
            };
            post.in_cluster(cluster.name.clone(), role)
        }
        None => post,
    }
}

fn body(plan: &Plan) -> PlanBody {
    PlanBody {
        per_week: plan.pace.get(),
        pace_from: match plan.pace_from {
            PaceFrom::Asked => "asked",
            _ => "history",
        },
        calendar: plan
            .calendar
            .iter()
            .map(|slot| SlotBody {
                day: slot.day.to_string(),
                id: slot.id.clone(),
                title: slot.title.clone(),
                kind: kind(slot.kind),
            })
            .collect(),
        waiting: plan
            .waiting
            .iter()
            .map(|wait| WaitingBody {
                id: wait.id.clone(),
                title: wait.title.clone(),
                why: why(&wait.why),
            })
            .collect(),
        warnings: plan.warnings.iter().map(warning).collect(),
    }
}

fn text(plan: &Plan) -> String {
    let mut text = String::new();
    let from = match plan.pace_from {
        PaceFrom::Asked => "as asked",
        _ => "the site's pace over the last twelve weeks",
    };
    let _ = writeln!(text, "Pace: {} a week, {from}.", plan.pace.get());
    if plan.calendar.is_empty() {
        text.push_str("\nNo post has a day yet.\n");
    } else {
        text.push_str("\nCalendar:\n");
    }
    for slot in &plan.calendar {
        let (day, kind, id, title) = (slot.day, kind(slot.kind), &slot.id, &slot.title);
        let _ = writeln!(text, "  {day}  {kind:<9}  {id}  {title}");
    }
    if !plan.waiting.is_empty() {
        text.push_str("\nWaiting:\n");
    }
    for wait in &plan.waiting {
        let _ = writeln!(text, "  {}  {}: {}", wait.id, wait.title, why(&wait.why));
    }
    for message in plan.warnings.iter().map(warning) {
        let _ = write!(text, "\n{message}\n");
    }
    text.push_str(
        "\nNothing was changed. Publish a post with `khalid publish`, or set its time in the admin.\n",
    );
    text
}

fn kind(kind: SlotKind) -> &'static str {
    match kind {
        SlotKind::Scheduled => "scheduled",
        _ => "suggested",
    }
}

fn why(why: &WaitReason) -> String {
    match why {
        WaitReason::PillarBlocked(pillar) => {
            format!("waits for its pillar {pillar}, which the linter blocks. Fix the pillar first.")
        }
        _ => "the linter blocks it. Run `khalid lint` on it to see why.".to_owned(),
    }
}

fn warning(warning: &PlanWarning) -> UserMessage {
    match warning {
        PlanWarning::PaceDropped { recent, before } => plan_message::pace_dropped(*recent, *before),
        PlanWarning::MemberBeforePillar { member, pillar } => {
            plan_message::member_first(member, pillar)
        }
        PlanWarning::NoneReady { last } => plan_message::none_ready(*last),
        _ => plan_message::unknown(),
    }
}
